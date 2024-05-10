//! This crate provides a library and a binary for tracing locks taken by SQL statements
//! in a PostgreSQL database. It can be used to analyze the locking behavior of SQL scripts
//! and to review migration scripts that could potentially interfere with other operations,
//! such as concurrent queries by application code.
use std::collections::HashMap;

use anyhow::anyhow;
use postgres::{Client, NoTls};
use tracing::trace_transaction;

use crate::sqltext::{read_sql_statements, resolve_placeholders, sql_statements};
use crate::tracing::TxLockTracer;

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
    let sql_statements = sql_statements(&sql_script)?;

    let all_concurrently = sql_statements.iter().all(sqltext::is_concurrently);

    let mut conn = Client::connect(connection_settings.connection_string().as_str(), NoTls)?;

    let result = if all_concurrently && trace.commit {
        for s in sql_statements.iter() {
            conn.execute(*s, &[])?;
        }
        TxLockTracer::tracer_for_concurrently(name, sql_statements.iter())
    } else {
        let mut tx = conn.transaction()?;

        // TODO: We probably need to special case create index concurrently here, since it's
        // illegal to run concurrently in a transaction, eg. we'd need to run it with auto-commit.

        let trace_result = trace_transaction(name, &mut tx, sql_statements.iter())?;

        if trace.commit {
            tx.commit()?;
        } else {
            tx.rollback()?;
        }

        trace_result
    };
    conn.close()?;
    Ok(result)
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

#[cfg(test)]
mod tests {
    use postgres::NoTls;

    use crate::{generate_new_test_db, ConnectionSettings};

    #[test]
    fn test_with_commit_we_can_run_concurrently_statements() {
        let trace_settings = super::TraceSettings {
            path: "examples/create_index_concurrently.sql".to_string(),
            commit: true,
            placeholders: Default::default(),
        };
        let connection_settings = ConnectionSettings::new(
            "postgres".to_string(),
            generate_new_test_db(),
            "localhost".to_string(),
            5432,
            "postgres".to_string(),
        );
        let mut conn =
            postgres::Client::connect(connection_settings.connection_string().as_str(), NoTls)
                .unwrap();
        // drop the index if it is already there
        conn.execute("DROP INDEX IF EXISTS books_concurrently_test_idx", &[])
            .unwrap();
        super::perform_trace(&trace_settings, &connection_settings).unwrap();

        let exists: bool = conn
            .query_one(
                "select count(*) > 0 from pg_class where relname = 'books_concurrently_test_idx'",
                &[],
            )
            .unwrap()
            .get(0);
        assert!(exists);
    }
}
