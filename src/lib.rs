//! This crate provides a library and a binary for tracing locks taken by SQL statements
//! in a PostgreSQL database. It can be used to analyze the locking behavior of SQL scripts
//! and to review migration scripts that could potentially interfere with other operations,
//! such as concurrent queries by application code.
use std::collections::HashMap;

use anyhow::anyhow;
use postgres::{Client, NoTls};

use crate::sqltext::{read_sql_statements, resolve_placeholders, sql_statements};
use crate::tracing::{trace_transaction, TxLockTracer};

/// Hints that can help avoid dangerous migrations, by minimizing time spent holding dangerous locks.
pub mod hints;
/// Generate output structures for lock traces and static data like lock modes.
/// This module is used by the binary to generate output in various formats is currently
/// the best documentation of output format, and can be considered a public api
/// for the library.
pub mod output;
/// Types that directly translate to postgres concepts like lock modes and relkinds.
pub mod pg_types;
/// Parse the postgres PGPASS file format.
pub mod pgpass;
/// Read and parse simple SQL scripts, resolve placeholders and break down into statements.
pub mod sqltext;
/// Trace locks taken by SQL statements. Structures and data from here should be considered
/// internal to the crate, their fields are not part of the public API.
pub mod tracing;

/// Connection settings for connecting to a PostgreSQL database.
pub struct ConnectionSettings {
    user: String,
    database: String,
    host: String,
    port: u16,
    password: String,
}

impl ConnectionSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "host={} user={} dbname={} port={} password={}",
            self.host, self.user, self.database, self.port, self.password
        )
    }
    pub fn new(user: String, database: String, host: String, port: u16, password: String) -> Self {
        ConnectionSettings {
            user,
            database,
            host,
            port,
            password,
        }
    }
}

/// Settings for tracing locks taken by SQL statements.
pub struct TraceSettings<'a> {
    path: String,
    commit: bool,
    placeholders: HashMap<&'a str, &'a str>,
}

impl<'a> TraceSettings<'a> {
    /// Create a new TraceSettings instance.
    /// # Arguments
    /// * `path` - Path to the SQL script to trace, or "-" to read from stdin.
    /// * `commit` - Whether to commit the transaction at the end of the trace.
    /// * `placeholders` - `${}`-Placeholders to replace in the SQL script, provided
    ///   as a slice of strings in the form of `"name=value"`.
    pub fn new(
        path: String,
        commit: bool,
        placeholders: &'a [String],
    ) -> Result<TraceSettings<'a>, anyhow::Error> {
        Ok(TraceSettings {
            path,
            commit,
            placeholders: parse_placeholders(placeholders)?,
        })
    }
}

/// Parse placeholders in the form of name=value into a map.
pub fn parse_placeholders(placeholders: &[String]) -> anyhow::Result<HashMap<&str, &str>> {
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

/// Perform a lock trace of a SQL script and optionally commit the transaction, depending on
/// trace_settings.
pub fn perform_trace(
    trace: &TraceSettings,
    connection_settings: &ConnectionSettings,
) -> anyhow::Result<TxLockTracer> {
    let script_content = read_sql_statements(&trace.path)?;
    let name = if trace.path == "-" {
        None
    } else {
        trace.path.split('/').last().map(|s| s.to_string())
    };
    let sql_script = resolve_placeholders(&script_content, &trace.placeholders)?;
    let sql_statements = sql_statements(&sql_script);
    let mut conn = Client::connect(connection_settings.connection_string().as_str(), NoTls)?;
    let mut tx = conn.transaction()?;
    let trace_result = trace_transaction(name, &mut tx, sql_statements.iter())?;
    if trace.commit {
        tx.commit()?;
    } else {
        tx.rollback()?;
    }
    conn.close()?;
    Ok(trace_result)
}
