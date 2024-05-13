use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;

use eugene::lints::apply_ignore_list;
use eugene::output::output_format::GenericHint;
use eugene::output::{DetailedLockMode, LockModesWrapper, TerseLockMode};
use eugene::pg_types::lock_modes;
use eugene::pgpass::read_pgpass_file;
use eugene::sqltext::{read_sql_statements, resolve_placeholders};
use eugene::{output, parse_placeholders, perform_trace, ConnectionSettings, TraceSettings};

#[derive(Parser)]
#[command(name = "eugene")]
#[command(about = "Careful with That Lock, Eugene")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    long_about = "eugene is a proof of concept tool for detecting dangerous locks taken by SQL migration scripts

eugene can run your migration scripts and detect which locks that is taken by each individual SQL statement and
summarize which operations that conflict with those locks, in other words what the script must wait for and what
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
    /// Lint SQL migration script by analyzing syntax tree and matching rules instead of running it.
    Lint {
        /// Path to SQL migration script, or '-' to read from stdin
        path: String,
        /// Provide name=value for replacing ${name} with value in the SQL script. Can be used multiple times.
        #[arg(short = 'v', long = "var")]
        placeholders: Vec<String>,
        /// Ignore the hints with these IDs, use `eugene hints` to see available hints. Can be used multiple times.
        ///
        /// Example: `eugene lint -i E3 -i E4`
        ///
        /// For finer granularity, you can annotate a SQL statement with an ignore-instruction like this:
        ///
        /// -- eugene-ignore: E3, E4
        ///
        /// alter table foo add column bar json;
        ///
        /// This will ignore hints E3 and E4 for this statement only.
        #[arg(short = 'i', long = "ignore")]
        ignored_hints: Vec<String>,
    },
    /// Trace locks taken by statements SQL migration script. Reads password from $PGPASS environment variable.
    Trace {
        /// Path to SQL migration script, or '-' to read from stdin
        path: String,
        /// Commit at the end of the transaction. Roll back by default.
        #[arg(short = 'c', long = "commit", default_value_t = false)]
        commit: bool,
        /// Provide name=value for replacing ${name} with value in the SQL script. Can be used multiple times.
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
        #[arg(short = 'f', long = "format", default_value = "json")]
        format: String,
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

fn trace(
    provided_connection_settings: ProvidedConnectionSettings,
    placeholders: Vec<String>,
    commit: bool,
    path: String,
    extra: bool,
    skip_summary: bool,
    trace_format: TraceFormat,
) -> Result<String> {
    let connection_settings = provided_connection_settings.try_into()?;
    let trace_settings = TraceSettings::new(path, commit, &placeholders)?;
    let trace_result = perform_trace(&trace_settings, &connection_settings)?;
    let full_trace =
        output::full_trace_data(&trace_result, output::Settings::new(!extra, skip_summary));
    match trace_format {
        TraceFormat::Json => full_trace.to_pretty_json(),
        TraceFormat::Plain => full_trace.to_plain_text(),
        TraceFormat::Markdown => full_trace.to_markdown(),
    }
}

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
            path,
            placeholders,
            ignored_hints,
        }) => {
            let sql = read_sql_statements(&path)?;
            let placeholders = parse_placeholders(&placeholders)?;
            let sql = resolve_placeholders(&sql, &placeholders)?;
            let report = eugene::lints::lint(sql)?;
            let report = apply_ignore_list(&report, &ignored_hints);
            let out = serde_json::to_string_pretty(&report)?;
            println!("{}", out);
            Ok(())
        }
        Some(Commands::Trace {
            user,
            database,
            host,
            port,
            placeholders,
            commit,
            path,
            extra,
            skip_summary,
            format,
        }) => {
            let out = trace(
                ProvidedConnectionSettings::new(user, database, host, port),
                placeholders,
                commit,
                path,
                extra,
                skip_summary,
                format.try_into()?,
            )?;
            println!("{}", out);
            Ok(())
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
            let hints: Vec<_> = eugene::hints::all_hints()
                .iter()
                .map(GenericHint::from)
                .collect();
            let hints = HintContainer { hints };
            println!("{}", serde_json::to_string_pretty(&hints)?);
            Ok(())
        }
    }
}
