use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use itertools::Itertools;
use serde::Serialize;

use eugene::output::output_format::GenericHint;
use eugene::output::{DetailedLockMode, LockModesWrapper, TerseLockMode};
use eugene::pg_types::lock_modes;
use eugene::pgpass::read_pgpass_file;
use eugene::script_discovery::script_filters;
use eugene::sqltext::resolve_placeholders;
use eugene::{
    output, parse_placeholders, perform_trace, script_discovery, ConnectionSettings, TraceSettings,
};

#[derive(Parser)]
#[command(name = "eugene")]
#[command(about = "Careful with That Lock, Eugene")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    long_about = "eugene is a tool for writing safer schema changes for PostgreSQL

eugene can run your migration scripts and detect which locks that is taken by each
individual SQL statement and summarize which operations that conflict with those
locks, in other words what the script must wait for and what
concurrent transactions that would be blocked.
"
)]
struct Eugene {
    /// Output format, plain, json
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Lint SQL migration script by analyzing syntax tree
    ///
    /// `eugene lint` exits with failure if any lint is detected.
    Lint {
        /// Path to SQL migration scripts, directories, or '-' to read from stdin
        #[arg(name = "paths")]
        paths: Vec<String>,
        /// Provide name=value for replacing ${name} with value in the SQL script
        ///
        /// Can be used multiple times to provide more placeholders.
        #[arg(short = 'v', long = "var")]
        placeholders: Vec<String>,
        /// Ignore the hints with these IDs, use `eugene hints` to see available hints
        ///
        /// Can be used multiple times.
        ///
        /// Example: `eugene lint -i E3 -i E4`
        ///
        /// Or comment your SQL statement like this:
        ///
        /// `-- eugene-ignore: E3, E4`
        ///
        /// alter table foo add column bar json;
        ///
        /// This will ignore hints E3 and E4 for this statement only.
        #[arg(short = 'i', long = "ignore")]
        ignored_hints: Vec<String>,
        /// Output format, plain, json or markdown
        #[arg(short = 'f', long = "format", default_value = "json", value_parser=clap::builder::PossibleValuesParser::new(["json", "markdown", "md"]))]
        format: String,
        /// Exit successfully even if problems are detected.
        ///
        /// Will still fail for errors in the SQL script.
        #[arg(short = 'a', long = "accept-failures", default_value_t = false)]
        accept_failures: bool,
    },
    /// Trace effects by running statements from SQL migration script
    ///
    /// Reads $PGPASS for password to postgres, if ~/.pgpass is not found.
    ///
    /// `eugene trace` exits with failure if any problems are detected.
    Trace {
        /// Path to SQL migration script, directories, or '-' to read from stdin
        #[arg(name = "paths")]
        paths: Vec<String>,
        /// Commit at the end of the transaction. Roll back by default.
        #[arg(short = 'c', long = "commit", default_value_t = false)]
        commit: bool,
        /// Provide name=value for replacing ${name} with value in the SQL script
        ///
        /// Can be used multiple times to provide more placeholders.
        #[arg(short = 'v', long = "var")]
        placeholders: Vec<String>,
        /// Username to use for connecting to postgres
        #[arg(short = 'U', long = "user", default_value = "postgres")]
        user: String,
        /// Database to connect to.
        #[arg(short = 'd', long = "database", default_value = "postgres")]
        database: String,
        /// Host to connect to.
        #[arg(short = 'H', long = "host", default_value = "localhost")]
        host: String,
        /// Port to connect to.
        #[arg(short = 'p', long = "port", default_value = "5432")]
        port: u16,

        /// Show locks that are normally not in conflict with application code.
        #[arg(short = 'e', long = "extra", default_value_t = false)]
        extra: bool,
        /// Skip the summary section for markdown output
        #[arg(short = 's', long = "skip-summary", default_value_t = false)]
        skip_summary: bool,
        /// Output format, plain, json or markdown
        #[arg(short = 'f', long = "format", default_value = "json", value_parser=clap::builder::PossibleValuesParser::new(["json", "markdown", "md", "plain"]))]
        format: String,
        /// Ignore the hints with these IDs, use `eugene hints` to see available hints
        ///
        /// Can be used multiple times.
        ///
        /// Example: `eugene trace -i E3 -i E4`
        ///
        /// Or comment your SQL statement like this to ignore for a single statement:
        ///
        /// -- eugene: ignore E4
        ///
        /// alter table foo add column bar json;
        ///
        /// Use `-- eugene: ignore` to ignore all hints for a statement.
        #[arg(short = 'i', long = "ignore")]
        ignored_hints: Vec<String>,
        /// Exit successfully even if problems are detected
        ///
        /// Will still fail for invalid SQL or connection problems.
        #[arg(short = 'a', long = "accept-failures", default_value_t = false)]
        accept_failures: bool,
    },
    /// List postgres lock modes
    Modes {
        /// Output format, json
        #[arg(short = 'f', long = "format", default_value = "json")]
        format: String,
    },
    /// Explain what operations a lock mode allows and conflicts with
    Explain {
        /// Lock mode to explain
        mode: String,
        /// Output format, json
        #[arg(short = 'f', long = "format", default_value = "json")]
        format: String,
    },
    /// Show migration hints that eugene can detect in traces
    Hints {
        /// Output format, json
        #[arg(short = 'f', long = "format", default_value = "json")]
        format: String,
    },
}

struct ProvidedConnectionSettings {
    user: String,
    database: String,
    host: String,
    port: u16,
}

impl ProvidedConnectionSettings {
    fn new(user: String, database: String, host: String, port: u16) -> Self {
        ProvidedConnectionSettings {
            user,
            database,
            host,
            port,
        }
    }
}

impl TryFrom<ProvidedConnectionSettings> for ConnectionSettings {
    type Error = anyhow::Error;

    fn try_from(value: ProvidedConnectionSettings) -> std::result::Result<Self, Self::Error> {
        let password = if let Ok(password) = std::env::var("PGPASS") {
            password
        } else {
            read_pgpass_file()?
                .find_password(&value.host, value.port, &value.database, &value.user)
                .context("No password found, provide PGPASS as environment variable or set up pgpassfile: https://www.postgresql.org/docs/current/libpq-pgpass.html")?
                .to_string()
        };
        Ok(ConnectionSettings::new(
            value.user,
            value.database,
            value.host,
            value.port,
            password,
        ))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct TraceConfiguration {
    trace_format: TraceFormat,
    extra_lock_info: bool,
    skip_summary: bool,
    ignored_hints: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum TraceFormat {
    Json,
    Plain,
    Markdown,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct HintContainer {
    hints: Vec<GenericHint>,
}

impl TryFrom<String> for TraceFormat {
    type Error = anyhow::Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.as_str() {
            "json" => Ok(TraceFormat::Json),
            "plain" => Ok(TraceFormat::Plain),
            "md" | "markdown" => Ok(TraceFormat::Markdown),
            _ => Err(anyhow!(
                "Invalid trace format: {}, possible choices: {:?}",
                value,
                &["json", "plain", "markdown"]
            )),
        }
    }
}

pub fn main() -> Result<()> {
    let args = Eugene::parse();
    match args.command {
        Some(Commands::Lint {
            paths,
            placeholders,
            ignored_hints,
            format,
            accept_failures: exit_success,
        }) => {
            let placeholders = parse_placeholders(&placeholders)?;
            let format: TraceFormat = format.try_into()?;
            let mut failed = false;
            for target in paths {
                for read_from in
                    script_discovery::discover_scripts(&target, script_filters::never, true)?
                {
                    let sql = read_from.read()?;
                    let name = read_from.name();
                    let sql = resolve_placeholders(&sql, &placeholders)?;
                    let report = eugene::lints::lint(
                        Some(name.to_string()),
                        sql,
                        &ignored_hints.iter().map(|s| s.as_str()).collect_vec(),
                    )?;
                    failed = failed || report.lints.iter().any(|stmt| !stmt.lints.is_empty());
                    let out = if matches!(format, TraceFormat::Json) {
                        serde_json::to_string_pretty(&report)?
                    } else {
                        output::markdown::lint_report_to_markdown(&report)?
                    };
                    println!("{}", out);
                }
            }
            if failed && !exit_success {
                Err(anyhow!("Lint detected"))
            } else {
                Ok(())
            }
        }
        Some(Commands::Trace {
            user,
            database,
            host,
            port,
            placeholders,
            commit,
            paths,
            extra,
            skip_summary,
            format,
            ignored_hints,
            accept_failures: exit_success,
        }) => {
            let config = TraceConfiguration {
                trace_format: format.try_into()?,
                extra_lock_info: extra,
                skip_summary,
                ignored_hints,
            };
            let mut connection_settings =
                ProvidedConnectionSettings::new(user, database, host, port).try_into()?;
            let mut failed = false;
            let placeholders = parse_placeholders(&placeholders)?;
            let ignore_list = config
                .ignored_hints
                .iter()
                .map(|s| s.as_str())
                .collect_vec();

            for target in paths {
                let script_source = script_discovery::discover_scripts(
                    &target,
                    script_filters::skip_downgrade_and_repeatable,
                    true,
                )?;
                for read_from in script_source {
                    let sql = read_from.read()?;
                    let sql = resolve_placeholders(&sql, &placeholders)?;
                    let name = read_from.name();
                    let trace_settings = TraceSettings::new(name.to_string(), &sql, commit);
                    let trace =
                        perform_trace(&trace_settings, &mut connection_settings, &ignore_list)?;
                    let full_trace = output::full_trace_data(
                        &trace,
                        output::Settings::new(!config.extra_lock_info, config.skip_summary),
                    );
                    failed = failed || !trace.success();
                    let report = match config.trace_format {
                        TraceFormat::Json => full_trace.to_pretty_json(),
                        TraceFormat::Plain => full_trace.to_plain_text(),
                        TraceFormat::Markdown => full_trace.to_markdown(),
                    }?;
                    println!("{}", report);
                }
            }

            if failed || !exit_success {
                Err(anyhow!("Trace uncovered problems"))
            } else {
                Ok(())
            }
        }
        Some(Commands::Modes { .. }) | None => {
            let lock_modes: Vec<_> = lock_modes::LOCK_MODES
                .iter()
                .map(TerseLockMode::from)
                .collect();
            let wrapper = LockModesWrapper::new(lock_modes);
            println!("{}", serde_json::to_string_pretty(&wrapper)?);
            Ok(())
        }
        Some(Commands::Explain { mode, .. }) => {
            let choice = lock_modes::LOCK_MODES
                .iter()
                .find(|m| m.to_db_str() == mode || m.to_db_str().replace("Lock", "") == mode)
                .context(format!("Invalid lock mode {mode}"))?;
            let choice: DetailedLockMode = choice.into();
            println!("{}", serde_json::to_string_pretty(&choice)?);
            Ok(())
        }
        Some(Commands::Hints { .. }) => {
            let hints: Vec<_> = eugene::hint_data::ALL
                .iter()
                .copied()
                .map(GenericHint::from)
                .collect();
            let hints = HintContainer { hints };
            println!("{}", serde_json::to_string_pretty(&hints)?);
            Ok(())
        }
    }
}
