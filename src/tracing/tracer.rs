use std::collections::HashSet;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use postgres::types::Oid;
use postgres::Transaction;

use crate::pg_types::locks::{Lock, LockableTarget};

/// A trace of a single SQL statement, including the locks taken and the duration of the statement.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct SqlStatementTrace {
    /// The SQL statement that was executed.
    pub(crate) sql: String,
    /// New locks taken by this statement.
    pub(crate) locks_taken: Vec<Lock>,
    /// The time the statement started executing.
    pub(crate) start_time: Instant,
    /// The duration of the statement.
    pub(crate) duration: Duration,
}

/// Enumerate all locks owned by the current transaction.
fn query_pg_locks_in_current_transaction(tx: &mut Transaction) -> Result<HashSet<Lock>> {
    let query = "SELECT n.nspname::text AS schema_name,
                c.relname::text AS object_name,
                c.relkind AS relkind,
                l.mode::text AS mode,
                c.oid AS oid
         FROM pg_locks l JOIN pg_class c ON c.oid = l.relation
           JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE l.locktype = 'relation' AND l.pid = pg_backend_pid();";
    let rows = tx.query(query, &[])?;
    let locks = rows
        .into_iter()
        .map(|row| {
            let schema: String = row.try_get(0)?;
            let object_name: String = row.try_get(1)?;
            let relkind: i8 = row.try_get(2)?;
            let mode: String = row.try_get(3)?;
            let oid: Oid = row.try_get(4)?;
            Lock::new(schema, object_name, mode, (relkind as u8) as char, oid)
                .map_err(|err| anyhow!("{err}"))
        })
        .collect::<Result<HashSet<Lock>, anyhow::Error>>()?;
    Ok(locks)
}

/// Find all locks in the current transaction that are relevant to the given set of objects.
fn find_relevant_locks_in_current_transaction(
    tx: &mut Transaction,
    relevant_objects: &HashSet<Oid>,
) -> Result<HashSet<Lock>> {
    let current_locks = query_pg_locks_in_current_transaction(tx)?;
    Ok(current_locks
        .into_iter()
        .filter(|lock| relevant_objects.contains(&lock.target_oid()))
        .collect())
}

/// Return the locks that are new in the new set of locks compared to the old set.
fn find_new_locks(old_locks: &HashSet<Lock>, new_locks: &HashSet<Lock>) -> HashSet<Lock> {
    let old = old_locks
        .iter()
        .map(|lock| (lock.target_oid(), lock.mode))
        .collect::<HashSet<_>>();
    new_locks
        .iter()
        .filter(|lock| !old.contains(&(lock.target_oid(), lock.mode)))
        .cloned()
        .collect()
}

/// A trace of a transaction, including all SQL statements executed and the locks taken by each one.
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct TxLockTracer {
    /// The name of the transaction, if any, typically the file name.
    pub(crate) name: Option<String>,
    /// The initial set of objects that are interesting to track locks for.
    initial_objects: HashSet<Oid>,
    /// The list of all SQL statements executed so far in the transaction.
    pub(crate) statements: Vec<SqlStatementTrace>,
    /// All locks taken so far in the transaction.
    all_locks: HashSet<Lock>,
    /// The time the trace started
    pub(crate) trace_start: DateTime<Local>,
}

impl TxLockTracer {
    /// Trace a single SQL statement, recording the locks taken and the duration of the statement.
    pub fn trace_sql_statement(&mut self, tx: &mut Transaction, sql: &str) -> Result<()> {
        let start_time = Instant::now();
        tx.execute(sql, &[])
            .map_err(|err| anyhow!("{err} while executing {}", sql.to_owned()))?;
        let duration = start_time.elapsed();
        let locks_taken = find_relevant_locks_in_current_transaction(tx, &self.initial_objects)?;
        let new_locks = find_new_locks(&self.all_locks, &locks_taken);
        self.all_locks.extend(locks_taken.iter().cloned());
        self.statements.push(SqlStatementTrace {
            sql: sql.to_string(),
            locks_taken: new_locks.into_iter().collect(),
            start_time,
            duration,
        });
        Ok(())
    }
    pub fn new(name: Option<String>, initial_objects: HashSet<Oid>) -> Self {
        Self {
            name,
            initial_objects,
            statements: vec![],
            all_locks: HashSet::new(),
            trace_start: Local::now(),
        }
    }
}

/// Fetch all user owned lockable objects in the database, skipping the system schemas.
fn fetch_lockable_objects(tx: &mut Transaction) -> Result<HashSet<LockableTarget>, anyhow::Error> {
    let sql = "SELECT
           n.nspname as schema_name,
           c.relname as table_name,
           c.relkind as relkind,
           c.oid as oid
         FROM pg_catalog.pg_class c
           JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
         ";
    let rows = tx.query(sql, &[]).map_err(|err| anyhow!("{err}"))?;
    rows.into_iter()
        .map(|row| {
            let schema: String = row.try_get(0)?;
            let object_name: String = row.try_get(1)?;
            let rk_byte: i8 = row.try_get(2)?;
            let rel_kind: char = (rk_byte as u8) as char;
            let oid: Oid = row.try_get(3)?;
            LockableTarget::new(schema.as_str(), object_name.as_str(), rel_kind, oid).ok_or(
                anyhow!("{schema}.{object_name} has invalid relkind: {rel_kind}"),
            )
        })
        .collect()
}

/// Trace a transaction, executing a series of SQL statements and recording the locks taken.
pub fn trace_transaction<S: AsRef<str>>(
    name: Option<String>,
    tx: &mut Transaction,
    sql_statements: impl Iterator<Item = S>,
) -> Result<TxLockTracer> {
    let initial_objects = fetch_lockable_objects(tx)?
        .into_iter()
        .map(|obj| obj.oid)
        .collect();
    let mut trace = TxLockTracer::new(name, initial_objects);
    for sql in sql_statements {
        trace.trace_sql_statement(tx, sql.as_ref().trim())?;
    }
    Ok(trace)
}
