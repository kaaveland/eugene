use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};

use eugene::output::{DetailedLockMode, LockModesWrapper, TerseLockMode};
use eugene::pg_types::lock_modes;
use eugene::pgpass::read_pgpass_file;
use eugene::{output, perform_trace, ConnectionSettings, TraceSettings};

#[derive(Parser)]
#[command(name = "eugene")]
#[command(about = "Careful with That Lock, Eugene")]
#[command(version = "0.1.0")]
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

enum Formats {
    Json,
}

impl Formats {
    fn render_locks(&self, output: &LockModesWrapper<TerseLockMode>) -> Result<String> {
        match self {
            Formats::Json => Ok(serde_json::to_string_pretty(output)?),
        }
    }
    fn render_single_lock(&self, output: &DetailedLockMode) -> Result<String> {
        match self {
            Formats::Json => Ok(serde_json::to_string_pretty(output)?),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
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
        #[arg(short = 'f', long = "format", default_value = "json")]
        format: String,
    },
    /// List postgres lock modes
    Modes {
        #[arg(short = 'f', long = "format", default_value = "json")]
        format: String,
    },
    /// Explain what operations a lock mode allows and conflicts with
    Explain {
        /// Lock mode to explain
        mode: String,

        #[arg(short = 'f', long = "format", default_value = "json")]
        format: String,
    },
}

impl Commands {
    fn format(&self) -> Result<Formats> {
        match self {
            Commands::Trace { format, .. }
            | Commands::Modes { format, .. }
            | Commands::Explain { format, .. } => match format.as_str() {
                "json" => Ok(Formats::Json),
                _ => Err(anyhow!("Invalid format: {}", format)),
            },
        }
    }
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
            let pgpass = read_pgpass_file()?;
            pgpass
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
) -> Result<String> {
    let connection_settings = provided_connection_settings.try_into()?;
    let trace_settings = TraceSettings::new(path, commit, &placeholders)?;
    let trace_result = perform_trace(&trace_settings, &connection_settings)?;
    let full_trace = output::full_trace_data(&trace_result, output::Settings::new(!extra));
    Ok(serde_json::to_string_pretty(&full_trace)?)
}

pub fn main() -> Result<()> {
    let args = Eugene::parse();
    let format = args
        .command
        .as_ref()
        .map_or(Ok(Formats::Json), |c| c.format())?;

    match args.command {
        Some(Commands::Trace {
            user,
            database,
            host,
            port,
            placeholders,
            commit,
            path,
            extra,
            ..
        }) => {
            let out = trace(
                ProvidedConnectionSettings::new(user, database, host, port),
                placeholders,
                commit,
                path,
                extra,
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
            println!("{}", format.render_locks(&wrapper)?);
            Ok(())
        }
        Some(Commands::Explain { mode, .. }) => {
            let choice = lock_modes::LOCK_MODES
                .iter()
                .find(|m| m.to_db_str() == mode || m.to_db_str().replace("Lock", "") == mode)
                .context(format!("Invalid lock mode {mode}"))?;
            let choice: DetailedLockMode = choice.into();
            println!("{}", format.render_single_lock(&choice)?);
            Ok(())
        }
    }
}
