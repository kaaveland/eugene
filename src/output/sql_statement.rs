use serde::Serialize;

use crate::locks::Lock;
use crate::output::lock::{DetailedLock, NormalLock, TerseLock};
use crate::tracing::SqlStatementTrace;

pub struct SqlStatementCtx<'a> {
    pub(crate) trace: &'a SqlStatementTrace,
    pub(crate) statement_number: usize,
    pub(crate) locks_before: Vec<&'a Lock>,
    pub(crate) extra: bool,
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct StatementBase {
    statement_number: usize,
    duration_millis: u64,
}
impl<'a> From<&'a SqlStatementCtx<'a>> for StatementBase {
    fn from(value: &'a SqlStatementCtx) -> Self {
        StatementBase {
            statement_number: value.statement_number,
            duration_millis: value.trace.duration.as_millis() as u64,
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct TerseSqlStatement<'a> {
    #[serde(flatten)]
    statement_number: StatementBase,
    locks_taken: Vec<TerseLock<'a>>,
}

impl<'a> From<&'a SqlStatementCtx<'a>> for TerseSqlStatement<'a> {
    fn from(value: &'a SqlStatementCtx<'a>) -> Self {
        TerseSqlStatement {
            statement_number: value.into(),
            locks_taken: value
                .trace
                .locks_taken
                .iter()
                .filter(|lock| value.extra || lock.mode.dangerous())
                .map(|lock| lock.into())
                .collect(),
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct NormalSqlStatement<'a> {
    #[serde(flatten)]
    statement_number: StatementBase,
    sql: &'a str,
    locks_taken: Vec<NormalLock<'a>>,
    locks_held: Vec<NormalLock<'a>>,
}

impl<'a> From<&'a SqlStatementCtx<'a>> for NormalSqlStatement<'a> {
    fn from(value: &'a SqlStatementCtx<'a>) -> Self {
        NormalSqlStatement {
            statement_number: value.into(),
            sql: value.trace.sql.as_str(),
            locks_taken: value
                .trace
                .locks_taken
                .iter()
                .filter(|lock| value.extra || lock.mode.dangerous())
                .map(|lock| lock.into())
                .collect(),
            locks_held: value
                .locks_before
                .iter()
                .map(|lock| (*lock).into())
                .collect(),
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct VerboseSqlStatement<'a> {
    #[serde(flatten)]
    statement_number: StatementBase,
    sql: &'a str,
    locks_taken: Vec<DetailedLock<'a>>,
    locks_held: Vec<DetailedLock<'a>>,
}

impl<'a> From<&'a SqlStatementCtx<'a>> for VerboseSqlStatement<'a> {
    fn from(value: &'a SqlStatementCtx<'a>) -> Self {
        VerboseSqlStatement {
            statement_number: value.into(),
            sql: value.trace.sql.as_str(),
            locks_taken: value
                .trace
                .locks_taken
                .iter()
                .filter(|lock| value.extra || lock.mode.dangerous())
                .map(|lock| lock.into())
                .collect(),
            locks_held: value
                .locks_before
                .iter()
                .map(|lock| (*lock).into())
                .collect(),
        }
    }
}
