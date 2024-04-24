use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};

use eugene::output::{Detailed, JsonPretty, Normal, Renderer, Terse, TxTraceData};
use eugene::{perform_trace, ConnectionSettings, TraceSettings};
use eugene::pg_types::lock_modes;

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
    #[arg(short = 'f', long = "format", default_value = "plain")]
    format: String,
    #[command(subcommand)]
    command: Option<Commands>,
}

enum Level {
    Terse,
    Normal,
    Detailed,
}

impl TryFrom<&str> for Level {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "terse" => Ok(Level::Terse),
            "normal" => Ok(Level::Normal),
            "detailed" => Ok(Level::Detailed),
            _ => Err(anyhow!("Invalid level: {}", value)),
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

        /// Detail level: terse, normal, detailed
        #[arg(short = 'l', long = "level", default_value = "normal")]
        level: String,

        /// Show locks that are normally not in conflict with application code.
        #[arg(short = 'e', long = "extra", default_value_t = false)]
        extra: bool,
    },
    /// List postgres lock modes
    Modes {
        /// Detail level: terse, normal, detailed
        #[arg(short = 'l', long = "level", default_value = "terse")]
        level: String,
    },
    /// Explain what operations a lock mode allows and conflicts with
    Explain {
        /// Lock mode to explain
        mode: String,
        /// Detail level: terse, normal, detailed
        #[arg(short = 'l', long = "level", default_value = "detailed")]
        level: String,
    },
}

impl Commands {
    fn level(&self) -> Result<Level> {
        match self {
            Commands::Trace { level, .. }
            | Commands::Modes { level, .. }
            | Commands::Explain { level, .. } => Level::try_from(level.as_str()),
        }
    }
}

pub fn main() -> Result<()> {
    let args = Eugene::parse();
    let level = args
        .command
        .as_ref()
        .map_or(Ok(Level::Terse), |c| c.level())?;

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
            let password = std::env::var("PGPASS").context("No PGPASS environment variable set")?;
            let connection_settings = ConnectionSettings::new(user, database, host, port, password);
            let trace_settings = TraceSettings::new(path, commit, &placeholders)?;
            let trace_result = perform_trace(&trace_settings, &connection_settings)?;
            let selectable = TxTraceData::new(&trace_result, extra);
            let json = match level {
                Level::Terse => Terse.trace::<JsonPretty>(&selectable),
                Level::Normal => Normal.trace::<JsonPretty>(&selectable),
                Level::Detailed => Detailed.trace::<JsonPretty>(&selectable),
            }?;
            println!("{}", json);
            Ok(())
        }
        Some(Commands::Modes { .. }) | None => {
            let lock_modes: Vec<_> = lock_modes::LOCK_MODES.into_iter().collect();
            let json = match level {
                Level::Terse => Terse.lock_modes::<JsonPretty>(&lock_modes),
                Level::Normal => Normal.lock_modes::<JsonPretty>(&lock_modes),
                Level::Detailed => Detailed.lock_modes::<JsonPretty>(&lock_modes),
            }?;
            println!("{}", json);
            Ok(())
        }
        Some(Commands::Explain { mode, .. }) => {
            let choice = lock_modes::LOCK_MODES
                .iter()
                .find(|m| m.to_db_str() == mode || m.to_db_str().replace("Lock", "") == mode)
                .context(format!("Invalid lock mode {mode}"))?;
            let json = match level {
                Level::Terse => Terse.lock_mode::<JsonPretty>(choice),
                Level::Normal => Normal.lock_mode::<JsonPretty>(choice),
                Level::Detailed => Detailed.lock_mode::<JsonPretty>(choice),
            }?;
            println!("{}", json);
            Ok(())
        }
    }
}
