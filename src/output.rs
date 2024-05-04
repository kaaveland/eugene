use chrono::{DateTime, Local};
use serde::Serialize;

use crate::output::markdown_helpers::{theader, trow};
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

impl FullTraceData {
    /// Render a pretty-printed JSON representation of the trace.
    pub fn to_pretty_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(&self)?)
    }
    /// Render a terse terminal-friendly representation of the trace.
    pub fn to_plain_text(&self) -> anyhow::Result<String> {
        let mut result = String::new();
        result.push_str(&format!(
            "Trace of \"{}\", started at: {}\n",
            self.name.as_deref().unwrap_or("unnamed"),
            self.start_time.to_rfc3339()
        ));
        result.push_str(&format!(
            "Total duration: {} ms\n",
            self.total_duration_millis
        ));
        result.push_str("All locks acquired:\n");
        for lock in &self.all_locks_acquired {
            result.push_str(&format!("{}\n", serde_json::to_string(lock)?));
        }
        for statement in &self.statements {
            result.push_str(&format!(
                "Statement #{}:\n",
                statement.statement_number_in_transaction
            ));
            result.push_str(&format!("SQL: {}\n", statement.sql));
            result.push_str(&format!("Duration: {} ms\n", statement.duration_millis));
            result.push_str("Locks at start:\n");
            for lock in &statement.locks_at_start {
                result.push_str(&format!("{}\n", serde_json::to_string(lock)?));
            }
            result.push_str("New locks taken:\n");
            for lock in &statement.new_locks_taken {
                result.push_str(&format!("{}\n", serde_json::to_string(lock)?));
            }
        }
        Ok(result)
    }
    /// Render a markdown report suitable for human consumption from the trace.
    pub fn to_markdown(&self) -> anyhow::Result<String> {
        let mut result = String::new();
        result.push_str(&format!(
            "# Eugene 🔒 trace of `{}`\n\n",
            self.name.as_deref().unwrap_or("unnamed")
        ));
        result.push_str(
            "In the below trace, a lock is considered dangerous if it conflicts with application code \
             queries, such as `SELECT` or `INSERT`.\n\n");
        result.push_str("We should aim to hold dangerous locks for as little time as possible. \
        If a dangerous lock is held while doing an operation that does not require it, we should split the migration into two steps.\n\n");
        result.push_str("For example, it often will make sense to add a new column in one migration, then backfill it in a separate one, \
        since adding a column requires an `AccessExclusiveLock` while backfilling can do with a `RowExclusiveLock` which is much \
        less likely to block concurrent transactions.\n\n");

        result.push_str(&self.summary_section());
        for statement in self.statements.iter() {
            result.push_str(&Self::statement_section(statement));
        }
        Ok(result)
    }

    fn lock_header() -> String {
        theader(&["Schema", "Object", "Mode", "Relkind", "OID", "Safe"])
    }

    fn lock_row(lock: &TracedLock) -> String {
        trow(&[
            format!("`{}`", lock.schema).as_str(),
            format!("`{}`", lock.object_name).as_str(),
            format!("`{}`", lock.mode).as_str(),
            lock.relkind,
            lock.oid.to_string().as_str(),
            match lock.maybe_dangerous {
                true => "❌",
                false => "✅",
            },
        ])
    }

    fn statement_section(statement: &FullSqlStatementLockTrace) -> String {
        let mut result = String::new();
        result.push_str(&format!(
            "## Statement number {} for {} ms\n\n",
            statement.statement_number_in_transaction, statement.duration_millis
        ));
        result.push_str("### SQL\n\n");
        result.push_str("```sql\n");
        result.push_str(&statement.sql);
        result.push_str("\n```\n\n");
        result.push_str("### Locks at start\n\n");
        if statement.locks_at_start.is_empty() {
            result.push_str("No locks held at the start of this statement.\n\n");
        } else {
            result.push_str(Self::lock_header().as_str());
            for lock in statement.locks_at_start.iter() {
                result.push_str(Self::lock_row(lock).as_str());
            }
            result.push('\n');
        }
        result.push_str("### New locks taken\n\n");
        if statement.new_locks_taken.is_empty() {
            result.push_str("No new locks taken by this statement.\n\n");
        } else {
            result.push_str(&theader(&[
                "Schema", "Object", "Mode", "Relkind", "OID", "Safe",
            ]));
            for lock in statement.new_locks_taken.iter() {
                result.push_str(Self::lock_row(lock).as_str());
            }
        }
        result.push('\n');
        result
    }

    fn summary_section(&self) -> String {
        let mut result = String::new();
        result.push_str("## Overall Summary\n\n");
        let headers = [
            "Started at",
            "Total duration (ms)",
            "Number of dangerous locks",
        ];
        result.push_str(&theader(&headers));
        let dangerous_locks = self
            .all_locks_acquired
            .iter()
            .filter(|lock| lock.maybe_dangerous)
            .count();

        result.push_str(&trow(&[
            self.start_time.to_rfc3339().as_str(),
            self.total_duration_millis.to_string().as_str(),
            match dangerous_locks {
                0 => "0 ✅".to_string(),
                n => format!("{} ❌", n),
            }
            .as_str(),
        ]));

        if self.all_locks_acquired.is_empty() {
            result.push_str("\nNo locks acquired on database objects that already exist.\n\n");
        } else {
            result.push_str("\n\n### All locks acquired\n\n");
            result.push_str(&theader(&[
                "Schema",
                "Object",
                "Mode",
                "Relkind",
                "OID",
                "Safe",
                "Duration held (ms)",
            ]));
            let mut time_diff = 0;
            for statement in self.statements.iter() {
                for lock in statement.new_locks_taken.iter() {
                    result.push_str(&trow(&[
                        format!("`{}`", lock.schema).as_str(),
                        format!("`{}`", lock.object_name).as_str(),
                        format!("`{}`", lock.mode).as_str(),
                        lock.relkind,
                        lock.oid.to_string().as_str(),
                        match lock.maybe_dangerous {
                            true => "❌",
                            false => "✅",
                        },
                        (self.total_duration_millis - time_diff)
                            .to_string()
                            .as_str(),
                    ]));
                }
                time_diff += statement.duration_millis;
            }
            if dangerous_locks > 0 {
                result.push_str("### Dangerous locks found\n\n");
                for lock in self
                    .all_locks_acquired
                    .iter()
                    .filter(|lock| lock.maybe_dangerous)
                {
                    result.push_str(&format!(
                        "- `{}` would block the following operations on `{}.{}`:\n",
                        lock.mode, lock.schema, lock.object_name
                    ));
                    for query in lock.blocked_queries.iter() {
                        result.push_str(&format!("  + `{}`\n", query));
                    }
                }
            }
        }
        result + "\n"
    }
}

mod markdown_helpers {
    pub fn theader(header: &[&str]) -> String {
        let h = header.join(" | ");
        let dashes = header
            .iter()
            .map(|h| ["-"].repeat(h.len()).join(""))
            .collect::<Vec<_>>()
            .join(" | ");
        format!("{}\n{}\n", h, dashes)
    }

    pub fn trow(row: &[&str]) -> String {
        row.join(" | ") + "\n"
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
