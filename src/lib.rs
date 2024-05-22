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

use anyhow::anyhow;
use postgres::{Client, NoTls, Transaction};

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

/// Connection settings for connecting to a PostgreSQL database.
pub struct ConnectionSettings {
    user: String,
    database: String,
    host: String,
    port: u16,
    password: String,
    client: Option<Client>,
}

impl ConnectionSettings {
    pub fn connection_string(&self) -> String {
        let out = format!(
            "host={} user={} dbname={} port={} password={}",
            self.host, self.user, self.database, self.port, self.password
        );
        out
    }
    pub fn new(user: String, database: String, host: String, port: u16, password: String) -> Self {
        ConnectionSettings {
            user,
            database,
            host,
            port,
            password,
            client: None,
        }
    }

    pub fn with_client<T>(
        &mut self,
        f: impl FnOnce(&mut Client) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        if let Some(ref mut client) = self.client {
            f(client)
        } else {
            let client = Client::connect(self.connection_string().as_str(), NoTls)?;
            self.client = Some(client);
            f(self.client.as_mut().unwrap())
        }
    }

    pub fn in_transaction<T>(
        &mut self,
        commit: bool,
        f: impl FnOnce(&mut Transaction) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        self.with_client(|client| {
            let mut tx = client.transaction()?;
            let result = f(&mut tx)?;
            if commit {
                tx.commit()?;
            } else {
                tx.rollback()?;
            }
            Ok(result)
        })
    }
}

/// Settings for tracing locks taken by SQL statements.
pub struct TraceSettings<'a> {
    name: String,
    sql: &'a str,
    commit: bool,
}

impl<'a> TraceSettings<'a> {
    /// Create a new TraceSettings instance.
    pub fn new(name: String, sql: &'a str, commit: bool) -> TraceSettings<'a> {
        TraceSettings { name, sql, commit }
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
pub fn perform_trace<'a>(
    trace: &TraceSettings,
    connection_settings: &mut ConnectionSettings,
    ignored_hints: &'a [&'a str],
) -> anyhow::Result<TxLockTracer<'a>> {
    let sql_statements = sql_statements(trace.sql)?;
    let all_concurrently = sql_statements.iter().all(sqltext::is_concurrently);
    if all_concurrently && trace.commit {
        connection_settings.with_client(|client| {
            for s in sql_statements.iter() {
                client.execute(*s, &[])?;
            }
            Ok(())
        })?;

        Ok(TxLockTracer::tracer_for_concurrently(
            Some(trace.name.clone()),
            sql_statements.iter(),
            ignored_hints,
        ))
    } else {
        connection_settings.in_transaction(trace.commit, |conn| {
            trace_transaction(
                Some(trace.name.clone()),
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
