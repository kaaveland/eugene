use std::collections::HashMap;
use std::fmt::Display;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use postgres::{Client, NoTls};

use crate::sqltext::{read_sql_statements, resolve_placeholders, sql_statements};
use crate::tracer::{trace_transaction, TxLockTrace};

mod lock_modes;
mod locks;
mod relkinds;
mod sqltext;
mod tracer;

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

/// Use to render lock mode information to output
#[derive(Debug, Eq, PartialEq, Clone)]
struct LockModeInfo<'a> {
    lock_mode: &'a str,
    enabled_operations: &'a [&'a str],
    conflicts_with: &'a [&'a str],
    blocked_olap_operations: &'a [&'a str],
    blocked_ddl_operations: &'a [&'a str],
}

impl Display for LockModeInfo<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Lock mode: {}\n", self.lock_mode)?;
        write!(f, "Used for: {}\n", self.enabled_operations.join(", "))?;
        write!(f, "Conflicts with: {}\n", self.conflicts_with.join(", "))?;
        write!(
            f,
            "Blocked query types: {}\n",
            self.blocked_olap_operations.join(", ")
        )?;
        write!(
            f,
            "Blocked DDL operations: {}",
            self.blocked_ddl_operations.join(", ")
        )
    }
}

struct ConnectionSettings {
    user: String,
    database: String,
    host: String,
    port: u16,
    password: String,
}

impl ConnectionSettings {
    fn connection_string(&self) -> String {
        format!(
            "host={} user={} dbname={} port={} password={}",
            self.host, self.user, self.database, self.port, self.password
        )
    }
}

struct TraceSettings<'a> {
    path: String,
    commit: bool,
    placeholders: HashMap<&'a str, &'a str>,
}

fn parse_placeholders(placeholders: &[String]) -> Result<HashMap<&str, &str>> {
    let mut map = HashMap::new();
    for placeholder in placeholders {
        let parts: Vec<&str> = placeholder.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid placeholder: {}", placeholder));
        }
        map.insert(parts[0], parts[1]);
    }
    Ok(map)
}

pub fn perform_trace(
    trace: &TraceSettings,
    connection_settings: &ConnectionSettings,
) -> Result<TxLockTrace> {
    let script_content = read_sql_statements(&trace.path)?;
    let sql_script = resolve_placeholders(&script_content, &trace.placeholders)?;
    let sql_statements = sql_statements(&sql_script);
    let mut conn = Client::connect(connection_settings.connection_string().as_str(), NoTls)?;
    let mut tx = conn.transaction()?;
    let trace_result = trace_transaction(&mut tx, sql_statements.iter())?;
    if trace.commit {
        tx.commit()?;
    } else {
        tx.rollback()?;
    }
    Ok(trace_result)
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
            let connection_settings = ConnectionSettings {
                user,
                database,
                host,
                port,
                password,
            };
            let trace_settings = TraceSettings {
                path,
                commit,
                placeholders: parse_placeholders(&placeholders)?,
            };
            let trace_result = perform_trace(&trace_settings, &connection_settings)?;
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
            let info = LockModeInfo {
                lock_mode: choice.to_db_str(),
                enabled_operations: choice.capabilities(),
                conflicts_with: &choice
                    .conflicts_with()
                    .iter()
                    .map(|m| m.to_db_str())
                    .collect::<Vec<_>>(),
                blocked_olap_operations: &choice.blocked_queries(),
                blocked_ddl_operations: &choice.blocked_ddl(),
            };
            println!("{info}");
            Ok(())
        }
    }
}
