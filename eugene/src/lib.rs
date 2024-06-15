//! This is the crate documentation for [eugene](https://kaveland.no/eugene).
//!
//! This crate provides a library and a binary for tracing locks taken by SQL statements
//! in a PostgreSQL database. It can be used to analyze the locking behavior of SQL scripts
//! and to review migration scripts that could potentially interfere with other operations,
//! such as concurrent queries by application code.
//!
//! THe library also provides syntax tree analysis for SQL scripts, so it can be used to
//! analyze migration scripts for potential issues before running them.
use std::collections::HashMap;

use postgres::{Client, NoTls, Transaction};

use crate::error::{ContextualError, InnerError};
use crate::script_discovery::ReadFrom;
use tracing::trace_transaction;

use crate::sqltext::sql_statements;
use crate::tracing::TxLockTracer;

/// Static data for hints and lints, used to identify them in output or input.
pub mod hint_data;
/// Hints that can help avoid dangerous migrations, by minimizing time spent holding dangerous locks.
pub mod hints;
/// Hints that can be trigged only by looking at the SQL script, without running it.
pub mod lints;
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

/// Walk the file system and list migration scripts in sorted order
pub mod script_discovery;

/// Internal module for parsing eugene comment intstructions
pub(crate) mod comments;

#[cfg(test)]
mod render_doc_snapshots;

/// Utilities for converting a single SQL file with multiple scripts into a list of scripts.
pub mod parse_scripts;
/// This module is for creating and destroying postgres
/// database instances for eugene to trace.
pub mod tempserver;

pub mod error;

/// Utilities for invoking git
pub mod git;

pub mod utils {
    use std::path::Path;

    pub trait FsyncDir {
        fn fsync(&self) -> Result<(), std::io::Error>;
    }

    impl<P: AsRef<Path>> FsyncDir for P {
        fn fsync(&self) -> Result<(), std::io::Error> {
            let dir = std::fs::File::open(self)?;
            dir.sync_all()
        }
    }
}

pub struct SqlScript {
    pub name: String,
    pub sql: String,
}

pub type Result<T> = std::result::Result<T, error::Error>;
/// Read a SQL script from a source and resolve placeholders.
///
/// # Arguments
///
/// * `read_from` - A source to read the SQL script from.
/// * `placeholders` - A map of placeholders to resolve if found in the SQL script.
pub fn read_script(read_from: &ReadFrom, placeholders: &HashMap<&str, &str>) -> Result<SqlScript> {
    let sql = read_from.read()?;
    let sql = sqltext::resolve_placeholders(&sql, placeholders)?;
    Ok(SqlScript {
        name: read_from.name().to_string(),
        sql,
    })
}

/// Connection settings for connecting to a PostgreSQL database.
pub struct ClientSource {
    user: String,
    database: String,
    host: String,
    port: u16,
    password: String,
    client: Option<Client>,
}

impl ClientSource {
    pub fn connection_string(&self) -> String {
        let out = format!(
            "host={} user={} dbname={} port={} password={}",
            self.host, self.user, self.database, self.port, self.password
        );
        out
    }
    pub fn new(user: String, database: String, host: String, port: u16, password: String) -> Self {
        ClientSource {
            user,
            database,
            host,
            port,
            password,
            client: None,
        }
    }
}

pub trait WithClient {
    fn with_client<T>(&mut self, f: impl FnOnce(&mut Client) -> Result<T>) -> Result<T>;

    fn in_transaction<T>(
        &mut self,
        commit: bool,
        f: impl FnOnce(&mut Transaction) -> Result<T>,
    ) -> Result<T> {
        self.with_client(|client| {
            let mut tx = client.transaction()?;
            let result = f(&mut tx)?;
            if commit {
                tx.commit()?;
            } else {
                tx.rollback()?;
            }
            client.execute("RESET ALL", &[])?;
            Ok(result)
        })
    }
}

impl WithClient for ClientSource {
    fn with_client<T>(&mut self, f: impl FnOnce(&mut Client) -> Result<T>) -> Result<T> {
        if let Some(ref mut client) = self.client {
            f(client)
        } else {
            let client = Client::connect(self.connection_string().as_str(), NoTls)?;
            self.client = Some(client);
            f(self.client.as_mut().unwrap())
        }
    }
}
/// Parse placeholders in the form of name=value into a map.
pub fn parse_placeholders(placeholders: &[String]) -> Result<HashMap<&str, &str>> {
    let mut map = HashMap::new();
    for placeholder in placeholders {
        let parts: Vec<&str> = placeholder.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(InnerError::PlaceholderSyntaxError.with_context(format!(
                "Placeholder '{}' must be in the form name=value",
                placeholder
            )));
        }
        map.insert(parts[0], parts[1]);
    }
    Ok(map)
}

/// Perform a lock trace of a SQL script and optionally commit the transaction, depending on
/// trace_settings.
pub fn perform_trace<'a, T: WithClient>(
    script: &SqlScript,
    connection_settings: &mut T,
    ignored_hints: &'a [&'a str],
    commit: bool,
) -> Result<TxLockTracer<'a>> {
    let sql_statements = sql_statements(script.sql.as_str())?;
    let all_concurrently = sql_statements.iter().all(sqltext::is_concurrently);
    if all_concurrently && commit {
        connection_settings.with_client(|client| {
            for s in sql_statements.iter() {
                client.execute(*s, &[])?;
            }
            Ok(())
        })?;

        Ok(TxLockTracer::tracer_for_concurrently(
            Some(script.name.clone()),
            sql_statements.iter(),
            ignored_hints,
        ))
    } else {
        connection_settings.in_transaction(commit, |conn| {
            trace_transaction(
                Some(script.name.clone()),
                conn,
                sql_statements.iter(),
                ignored_hints,
            )
        })
    }
}

#[cfg(test)]
/// Generate a new copy of the test_db database for testing.
pub fn generate_new_test_db() -> String {
    let mut pg_client = Client::connect(
        "host=localhost dbname=postgres password=postgres user=postgres",
        NoTls,
    )
    .unwrap();

    pg_client
        .execute(
            "CREATE TABLE IF NOT EXISTS test_dbs(\
        name text PRIMARY KEY, time timestamptz default now());",
            &[],
        )
        .ok();

    let db_name = format!(
        "eugene_testdb_{}",
        uuid::Uuid::new_v4().to_string().replace('-', "_")
    );
    pg_client
        .execute(
            "INSERT INTO test_dbs(name) VALUES($1);",
            &[&db_name.as_str()],
        )
        .unwrap();

    let old_dbs = pg_client
        .query(
            "SELECT name FROM test_dbs WHERE time < now() - interval '15 minutes';",
            &[],
        )
        .unwrap();

    for row in old_dbs {
        let db_name: String = row.get(0);
        pg_client
            .execute(&format!("DROP DATABASE IF EXISTS {}", db_name), &[])
            .unwrap();
        pg_client
            .execute(
                "DELETE FROM test_dbs WHERE name = $1;",
                &[&db_name.as_str()],
            )
            .unwrap();
    }

    pg_client
        .execute(
            &format!("CREATE DATABASE {} TEMPLATE test_db", db_name),
            &[],
        )
        .unwrap();
    db_name
}
