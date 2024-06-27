use itertools::Itertools;
use serde::ser::SerializeStruct;
use serde::Serialize;

pub use output_format::{
    Column, Constraint, DbObject, FullSqlStatementLockTrace, FullTraceData, GenericHint, Hint,
    LintReport, LintedStatement, ModifiedColumn, ModifiedConstraint, TracedLock,
};

use crate::pg_types::lock_modes::LockMode;
use crate::pg_types::locks::Lock;
use crate::tracing::{SqlStatementTrace, TxLockTracer};

/// Output types for the lock tracing library, exportable to JSON and public API.
///
/// The intention is to provide serialization and eventually deserialization for lock traces
/// using these record types.
pub mod output_format;
/// Markdown rendering utilities for lock traces and lints.
pub mod templates;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Settings {
    only_dangerous_locks: bool,
    skip_summary_section: bool,
}

impl Settings {
    pub fn new(only_dangerous_locks: bool, skip_summary_section: bool) -> Self {
        Settings {
            only_dangerous_locks,
            skip_summary_section,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct OutputContext {
    output_settings: Settings,
    statement_number: usize,
    held_locks_context: Vec<TracedLock>,
    duration_millis_so_far: u64,
    duration_millis_total: u64,
}

impl OutputContext {
    fn output_lock(&self, lock: &Lock) -> TracedLock {
        TracedLock {
            schema: lock.target.schema.clone(),
            object_name: lock.target.object_name.clone(),
            relkind: lock.target.rel_kind.as_str(),
            mode: lock.mode.to_db_str().to_string(),
            maybe_dangerous: lock.mode.dangerous(),
            oid: lock.target.oid,
            blocked_queries: lock.blocked_queries(),
            lock_duration_millis: self.duration_millis_total - self.duration_millis_so_far,
        }
    }

    fn output_statement(
        &mut self,
        statement: &SqlStatementTrace,
        hints: &[Hint],
    ) -> FullSqlStatementLockTrace {
        let locks_at_start: Vec<_> = self
            .held_locks_context
            .iter()
            .cloned()
            .sorted_by_key(|lock| {
                (
                    lock.schema.clone(),
                    lock.object_name.clone(),
                    lock.relkind,
                    lock.mode.clone(),
                )
            })
            .collect();

        let new_locks_taken: Vec<_> = statement
            .locks_taken
            .iter()
            .filter(|lock| !self.hide_lock(lock))
            .map(|lock| self.output_lock(lock))
            .filter(|lock| !locks_at_start.contains(lock))
            .sorted_by_key(|lock| {
                (
                    lock.schema.clone(),
                    lock.object_name.clone(),
                    lock.relkind,
                    lock.mode.clone(),
                )
            })
            .collect();

        let result = FullSqlStatementLockTrace {
            statement_number_in_transaction: self.statement_number,
            line_number: statement.line_no,
            sql: statement.sql.clone(),
            duration_millis: statement.duration.as_millis() as u64,
            start_time_millis: self.duration_millis_so_far,
            new_locks_taken,
            locks_at_start,
            new_columns: statement
                .added_columns
                .iter()
                .map(|(_, c)| Column::from(c))
                .collect(),
            altered_columns: statement
                .modified_columns
                .iter()
                .map(|(_, c)| ModifiedColumn::from(c))
                .collect(),
            new_constraints: statement
                .added_constraints
                .iter()
                .map(Constraint::from)
                .collect(),
            altered_constraints: statement
                .modified_constraints
                .iter()
                .map(|(_, c)| ModifiedConstraint::from(c))
                .collect(),
            new_objects: statement
                .created_objects
                .iter()
                .map(DbObject::from)
                .collect(),
            lock_timeout_millis: statement.lock_timeout_millis,
            triggered_rules: hints.to_vec(),
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
    pub fn new(output_settings: Settings, duration_millis_total: u64) -> Self {
        OutputContext {
            output_settings,
            statement_number: 1,
            held_locks_context: vec![],
            duration_millis_so_far: 0,
            duration_millis_total,
        }
    }
}

pub fn full_trace_data(trace: &TxLockTracer, output_settings: Settings) -> FullTraceData {
    let total_duration = trace
        .statements
        .iter()
        .map(|st| st.duration.as_millis() as u64)
        .sum();
    let mut context = OutputContext::new(output_settings, total_duration);
    let mut statements = vec![];
    for (i, statement) in trace.statements.iter().enumerate() {
        statements.push(context.output_statement(statement, &trace.triggered_hints[i]));
    }
    let passed_all_checks = statements.iter().all(|st| st.triggered_rules.is_empty());
    context.held_locks_context.sort_by_key(|lock| {
        (
            lock.schema.clone(),
            lock.object_name.clone(),
            lock.relkind,
            lock.mode.clone(),
        )
    });
    let dangerous_locks_count = context
        .held_locks_context
        .iter()
        .filter(|lock| lock.maybe_dangerous)
        .count();

    FullTraceData {
        name: trace.name.clone(),
        start_time: trace.trace_start,
        total_duration_millis: context.duration_millis_so_far,
        all_locks_acquired: context.held_locks_context,
        statements,
        skip_summary: output_settings.skip_summary_section,
        dangerous_locks_count,
        passed_all_checks,
    }
}

struct JsonTrace<'a> {
    data: &'a FullTraceData,
}

impl<'a> Serialize for JsonTrace<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        if !self.data.skip_summary {
            self.data.serialize(serializer)
        } else {
            let mut state = serializer.serialize_struct("FullTraceData", 2)?;
            state.serialize_field("name", &self.data.name)?;
            state.serialize_field("statements", &self.data.statements)?;
            state.end()
        }
    }
}

impl FullTraceData {
    /// Render a pretty-printed JSON representation of the trace.
    pub fn to_pretty_json(&self) -> crate::Result<String> {
        let out = JsonTrace { data: self };
        Ok(serde_json::to_string_pretty(&out)?)
    }
    /// Render a terse terminal-friendly representation of the trace.
    pub fn to_plain_text(&self) -> crate::Result<String> {
        templates::trace_text(self)
    }
    /// Render a markdown report suitable for human consumption from the trace.
    pub fn to_markdown(&self) -> crate::Result<String> {
        templates::to_markdown(self)
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
