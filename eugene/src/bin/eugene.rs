use anyhow::{anyhow, Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::generate;
use clap_complete::Shell::{Bash, Elvish, Fish, PowerShell, Zsh};
use itertools::Itertools;
use postgres::Client;
use serde::Serialize;

use eugene::output::output_format::GenericHint;
use eugene::output::{DetailedLockMode, LockModesWrapper, TerseLockMode};
use eugene::pg_types::lock_modes;
use eugene::pgpass::read_pgpass_file;
use eugene::script_discovery::script_filters;
use eugene::sqltext::resolve_placeholders;
use eugene::tempserver::TempServer;
use eugene::{
    output, parse_placeholders, perform_trace, script_discovery, ConnectionSettings, TraceSettings,
    WithClient,
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
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser)]
struct LintOptions {
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
    #[arg(short = 'f', long = "format", default_value = "plain", value_parser=clap::builder::PossibleValuesParser::new(["json", "markdown", "md", "plain"]))]
    format: String,
    /// Exit successfully even if problems are detected.
    ///
    /// Will still fail for errors in the SQL script.
    #[arg(short = 'a', long = "accept-failures", default_value_t = false)]
    accept_failures: bool,

    /// Sort mode for script discovery, auto, name or none
    ///
    /// This is used to order scripts when an argument contains many scripts.
    ///
    /// `auto` will sort by versions or sequence numbers.
    ///
    /// `auto` requires all files to have the same naming scheme.
    ///
    /// `name` will sort lexically by name.
    #[arg(long = "sort-mode", default_value = "auto", value_parser=clap::builder::PossibleValuesParser::new(["auto", "name", "none"]))]
    sort_mode: String,
    /// Skip the summary section for markdown output
    #[arg(short = 's', long = "skip-summary", default_value_t = false)]
    skip_summary: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Lint SQL migration script by analyzing syntax tree
    ///
    /// `eugene lint` exits with failure if any lint is detected.
    Lint {
        #[command(flatten)]
        opts: LintOptions,
    },
    /// Trace effects by running statements from SQL migration script
    ///
    /// Reads $PGPASS for password to postgres, if ~/.pgpass is not found.
    ///
    /// `eugene trace` exits with failure if any problems are detected.
    Trace {
        #[command(flatten)]
        opts: LintOptions,
        /// Commit at the end of the transaction. Roll back by default.
        #[arg(short = 'c', long = "commit", default_value_t = false)]
        commit: bool,
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
        /// Disable creation of temporary postgres server for tracing
        ///
        /// By default, trace will create a postgres server in a temporary directory
        ///
        /// This relies on having `initdb` and `pg_ctl` in PATH, which eugene images have.
        ///
        /// Eugene deletes the temporary database cluster when done tracing.
        #[arg(long = "disable-temporary", default_value_t = true)]
        temporary_postgres: bool,

        /// Portgres options to pass to the temporary postgres server
        ///
        /// Example: `eugene trace -o "-c fsync=off -c log_statement=all"`
        #[arg(short = 'o', long = "postgres-options", default_value = "")]
        postgres_options: String,

        /// Initdb options to pass when creating the temporary postgres server
        ///
        /// Example: `eugene trace --initdb "--encoding=UTF8"`
        ///
        /// Supply it more than once to add multiple options.
        #[arg(long = "initdb")]
        initdb_options: Vec<String>,
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

    /// Generate shell completions for eugene
    ///
    /// Add the output to your shell configuration file or the preferred location
    /// for completions.
    Completions {
        #[arg(short, long, default_value = "bash", value_parser=clap::builder::PossibleValuesParser::new(["bash", "zsh", "fish", "pwsh", "powershell"]))]
        shell: String,
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

enum ClientSource {
    TempDb(TempServer),
    Connect(ConnectionSettings),
}

impl WithClient for ClientSource {
    fn with_client<T>(&mut self, f: impl FnOnce(&mut Client) -> Result<T>) -> Result<T> {
        match self {
            ClientSource::TempDb(temp) => temp.with_client(f),
            ClientSource::Connect(settings) => settings.with_client(f),
        }
    }
}

pub fn main() -> Result<()> {
    env_logger::init();
    let args = Eugene::parse();
    match args.command {
        Some(Commands::Lint {
            opts:
                LintOptions {
                    paths,
                    placeholders,
                    ignored_hints,
                    format,
                    accept_failures: exit_success,
                    sort_mode,
                    skip_summary,
                },
        }) => {
            let placeholders = parse_placeholders(&placeholders)?;
            let format: TraceFormat = format.try_into()?;
            let mut failed = false;
            for read_from in
                script_discovery::discover_all(paths, script_filters::never, sort_mode.try_into()?)?
            {
                let sql = read_from.read()?;
                let name = read_from.name();
                let sql = resolve_placeholders(&sql, &placeholders)?;
                let report = eugene::lints::lint(
                    Some(name.to_string()),
                    sql,
                    &ignored_hints.iter().map(|s| s.as_str()).collect_vec(),
                    skip_summary,
                )?;
                failed = failed
                    || report
                        .statements
                        .iter()
                        .any(|stmt| !stmt.triggered_rules.is_empty());
                let out = match format {
                    TraceFormat::Json => Ok(serde_json::to_string_pretty(&report)?),
                    TraceFormat::Plain => output::templates::lint_text(&report),
                    TraceFormat::Markdown => output::templates::lint_report_to_markdown(&report),
                }?;
                if !out.trim().is_empty() {
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
            opts:
                LintOptions {
                    paths,
                    placeholders,
                    ignored_hints,
                    format,
                    accept_failures: exit_success,
                    sort_mode,
                    skip_summary,
                },
            user,
            database,
            host,
            port,
            commit,
            extra,
            temporary_postgres,
            postgres_options,
            initdb_options,
        }) => {
            let commit = commit || temporary_postgres;
            let config = TraceConfiguration {
                trace_format: format.try_into()?,
                extra_lock_info: extra,
                skip_summary,
                ignored_hints,
            };
            let provided = ProvidedConnectionSettings::new(user, database, host, port);
            let mut client_source = if temporary_postgres {
                ClientSource::TempDb(TempServer::new(postgres_options.as_str(), &initdb_options)?)
            } else {
                ClientSource::Connect(provided.try_into()?)
            };

            let mut failed = false;
            let placeholders = parse_placeholders(&placeholders)?;
            let ignore_list = config
                .ignored_hints
                .iter()
                .map(|s| s.as_str())
                .collect_vec();

            let script_source = script_discovery::discover_all(
                paths,
                script_filters::skip_downgrade_and_repeatable,
                sort_mode.try_into()?,
            )?;
            if !commit && script_source.len() > 1 {
                return Err(anyhow!(
                    "{} scripts detected, use --commit if you want to trace them in sequence",
                    script_source.len()
                ));
            }
            for read_from in script_source {
                let sql = read_from.read()?;
                let sql = resolve_placeholders(&sql, &placeholders)?;
                let name = read_from.name();
                let trace_settings = TraceSettings::new(name.to_string(), &sql, commit);
                let trace = perform_trace(&trace_settings, &mut client_source, &ignore_list)
                    .map_err(|e| anyhow!("Error tracing {name}: {e}"))?;
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
                if !report.trim().is_empty() {
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
        Some(Commands::Completions { shell }) => {
            let mut com = Eugene::command();
            match shell.as_str() {
                "bash" => {
                    generate(Bash, &mut com, "eugene", &mut std::io::stdout());
                    Ok(())
                }
                "zsh" => {
                    generate(Zsh, &mut com, "eugene", &mut std::io::stdout());
                    Ok(())
                }
                "fish" => {
                    generate(Fish, &mut com, "eugene", &mut std::io::stdout());
                    Ok(())
                }
                "powershell" | "pwsh" => {
                    generate(PowerShell, &mut com, "eugene", &mut std::io::stdout());
                    Ok(())
                }
                "elvish" => {
                    generate(Elvish, &mut com, "eugene", &mut std::io::stdout());
                    Ok(())
                }
                _ => Err(anyhow!("Unsupported shell: {shell}")),
            }?;
            Ok(())
        }
    }
}
