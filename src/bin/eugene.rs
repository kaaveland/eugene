use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use eugene::{ConnectionSettings, lock_modes, TraceSettings};
use eugene::lock_modes::LockModeInfo;

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
    },
    /// List postgres lock modes
    LockModes,
    /// Explain what operations a lock mode allows and conflicts with
    Explain {
        /// Lock mode to explain
        mode: String,
    },
}

pub fn main() -> Result<()> {
    let args = Eugene::parse();
    match args.command {
        Some(Commands::Trace {
            user,
            database,
            host,
            port,
            placeholders,
            commit,
            path,
        }) => {
            let password = std::env::var("PGPASS").context("No PGPASS environment variable set")?;
            let connection_settings = ConnectionSettings::new(user, database, host, port, password);
            let trace_settings = TraceSettings::new(path, commit, &placeholders)?;
            let trace_result = eugene::perform_trace(&trace_settings, &connection_settings)?;
            println!("{trace_result}");
            Ok(())
        }
        Some(Commands::LockModes) | None => {
            lock_modes::LOCK_MODES.iter().for_each(|mode| {
                println!("{}", mode.to_db_str());
            });
            Ok(())
        }
        Some(Commands::Explain { mode }) => {
            let choice = lock_modes::LOCK_MODES
                .iter()
                .find(|m| m.to_db_str() == mode || m.to_db_str().replace("Lock", "") == mode)
                .context(format!("Invalid lock mode {mode}"))?;
            let info = LockModeInfo::new(choice);
            println!("{info}");
            Ok(())
        }
    }
}
