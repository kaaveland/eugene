use chrono::{DateTime, Local};
use serde::Serialize;

use crate::pg_types::lock_modes::LockMode;
use crate::pg_types::locks::Lock;
use crate::tracing::{SqlStatementTrace, TxLockTracer};

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Settings {
    only_dangerous_locks: bool,
}

impl Settings {
    pub fn new(only_dangerous_locks: bool) -> Self {
        Settings {
            only_dangerous_locks,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new(true)
    }
}

#[derive(Debug, Eq, PartialEq, Default)]
struct OutputContext {
    output_settings: Settings,
    statement_number: usize,
    held_locks_context: Vec<TracedLock>,
    duration_millis_so_far: u64,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
struct TracedLock {
    schema: String,
    object_name: String,
    mode: String,
    relkind: &'static str,
    oid: u32,
    maybe_dangerous: bool,
    blocked_queries: Vec<&'static str>,
    blocked_ddl: Vec<&'static str>,
}

fn traced_lock_from(lock: &Lock) -> TracedLock {
    TracedLock {
        schema: lock.target().schema.to_string(),
        object_name: lock.target().object_name.to_string(),
        mode: lock.mode.to_db_str().to_string(),
        relkind: lock.target().rel_kind.as_str(),
        oid: lock.target().oid,
        maybe_dangerous: lock.mode.dangerous(),
        blocked_queries: lock.blocked_queries(),
        blocked_ddl: lock.blocked_ddl(),
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
struct FullSqlStatementLockTrace {
    statement_number_in_transaction: usize,
    sql: String,
    duration_millis: u64,
    start_time_millis: u64,
    locks_at_start: Vec<TracedLock>,
    new_locks_taken: Vec<TracedLock>,
}

impl OutputContext {
    fn output_statement(&mut self, statement: &SqlStatementTrace) -> FullSqlStatementLockTrace {
        let locks_at_start = self.held_locks_context.clone();
        let new_locks_taken: Vec<_> = statement
            .locks_taken
            .iter()
            .filter(|lock| !self.hide_lock(lock))
            .map(traced_lock_from)
            .filter(|lock| !locks_at_start.contains(lock))
            .collect();
        let result = FullSqlStatementLockTrace {
            statement_number_in_transaction: self.statement_number,
            sql: statement.sql.clone(),
            duration_millis: statement.duration.as_millis() as u64,
            start_time_millis: self.duration_millis_so_far,
            new_locks_taken,
            locks_at_start,
        };
        self.statement_number += 1;
        self.held_locks_context
            .extend(result.new_locks_taken.clone());
        self.duration_millis_so_far += result.duration_millis;
        result
    }

    fn hide_lock(&self, lock: &Lock) -> bool {
        self.output_settings.only_dangerous_locks && !lock.mode.dangerous()
    }
    pub fn new(output_settings: Settings) -> Self {
        OutputContext {
            output_settings,
            statement_number: 1,
            ..OutputContext::default()
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct FullTraceData {
    name: Option<String>,
    #[serde(with = "datefmt")]
    start_time: DateTime<Local>,
    total_duration_millis: u64,
    all_locks_acquired: Vec<TracedLock>,
    statements: Vec<FullSqlStatementLockTrace>,
}

pub fn full_trace_data(trace: &TxLockTracer, output_settings: Settings) -> FullTraceData {
    let mut context = OutputContext::new(output_settings);
    let mut statements = vec![];
    for statement in &trace.statements {
        statements.push(context.output_statement(statement));
    }

    FullTraceData {
        name: trace.name.clone(),
        start_time: trace.trace_start,
        total_duration_millis: context.duration_millis_so_far,
        all_locks_acquired: context.held_locks_context,
        statements,
    }
}

mod datefmt {
    use chrono::{DateTime, Local};

    pub fn serialize<S>(date: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&date.to_rfc3339())
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct TerseLockMode<'a> {
    lock_mode: &'a str,
    #[serde(skip)]
    _phantom: std::marker::PhantomData<&'a LockMode>,
}

impl<'a> From<&'a LockMode> for TerseLockMode<'a> {
    fn from(value: &'a LockMode) -> Self {
        TerseLockMode {
            lock_mode: value.to_db_str(),
            _phantom: std::marker::PhantomData,
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct DetailedLockMode<'a> {
    #[serde(flatten)]
    terse: TerseLockMode<'a>,
    used_for: &'a [&'a str],
    conflicts_with: Vec<&'a str>,
    blocked_queries: Vec<&'a str>,
    blocked_ddl_operations: Vec<&'a str>,
}

impl<'a> From<&'a LockMode> for DetailedLockMode<'a> {
    fn from(value: &'a LockMode) -> Self {
        DetailedLockMode {
            terse: value.into(),
            used_for: value.capabilities(),
            conflicts_with: value
                .conflicts_with()
                .iter()
                .map(|s| s.to_db_str())
                .collect(),
            blocked_queries: value.blocked_queries(),
            blocked_ddl_operations: value.blocked_ddl(),
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct LockModesWrapper<L> {
    lock_modes: Vec<L>,
}

impl<L> LockModesWrapper<L> {
    pub fn new(lock_modes: Vec<L>) -> Self {
        LockModesWrapper { lock_modes }
    }
}
