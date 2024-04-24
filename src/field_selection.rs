use serde::Serialize;

use crate::field_selection::lock::{NormalLock, TerseLock, VerboseLock};
use crate::field_selection::lock_mode::{DetailedLockMode, NormalLockMode, TerseLockMode};
use crate::field_selection::sql_statement::{NormalSqlStatement, SqlStatementCtx, TerseSqlStatement, VerboseSqlStatement};
use crate::lock_modes::LockMode;
use crate::locks::Lock;
use crate::tracing::TxLockTracer;

mod lock;
mod lock_mode;
mod sql_statement;

/// Specialize this trait to render different levels of detail for different types
pub trait Renderer<'a> {
    type LockMode: Serialize + From<&'a LockMode>;
    type Lock: Serialize + From<&'a Lock>;
    type SqlStatement: Serialize + From<&'a SqlStatementCtx<'a>>;
    type TxTrace: Serialize + From<&'a TxTraceSerializable<'a>>;

    fn lock_mode<F: Format<'a>>(&self, mode: &'a LockMode) -> Result<String, anyhow::Error> {
        let obj: Self::LockMode = mode.into();
        F::render(&obj)
    }
    fn lock<F: Format<'a>>(&self, lock: &'a Lock) -> Result<String, anyhow::Error> {
        let obj: Self::Lock = lock.into();
        F::render(&obj)
    }
    fn statement<F: Format<'a>>(&self, statement: &'a SqlStatementCtx<'a>) -> Result<String, anyhow::Error> {
        let obj: Self::SqlStatement = statement.into();
        F::render(&obj)
    }
    fn trace<F: Format<'a>>(&self, trace: &'a TxTraceSerializable<'a>) -> Result<String, anyhow::Error> {
        let obj: Self::TxTrace = trace.into();
        F::render(&obj)
    }
    fn lock_modes<F: Format<'a>>(&self, modes: &'a[LockMode]) -> Result<String, anyhow::Error> {
        let obj: Vec<Self::LockMode> = modes.iter().map(|mode| mode.into()).collect();
        F::render(&obj)
    }
}

/// Terse selects the bare minimum of fields to display in output
pub struct Terse;

impl <'a> Renderer<'a> for Terse {
    type LockMode = TerseLockMode<'a>;
    type Lock = TerseLock<'a>;
    type SqlStatement = TerseSqlStatement<'a>;
    type TxTrace = TerseTxTrace<'a>;
}
/// Normal selects more fields than Terse without being verbose
pub struct Normal;

impl <'a> Renderer<'a> for Normal {
    type LockMode = NormalLockMode<'a>;
    type Lock = NormalLock<'a>;
    type SqlStatement = NormalSqlStatement<'a>;
    type TxTrace = NormalTxTrace<'a>;
}

/// Verbose selects all possible fields
pub struct Verbose;

impl <'a> Renderer<'a> for Verbose {
    type LockMode = DetailedLockMode<'a>;
    type Lock = VerboseLock<'a>;
    type SqlStatement = VerboseSqlStatement<'a>;
    type TxTrace = DetailedTxTrace<'a>;
}
/// Output data with [serde_json::to_string]
pub struct Json;
/// Output data with [serde_json::to_string_pretty]
pub struct JsonPretty;
/// Format selected fields into a string
pub trait Format<'a> {
    fn render<I: Serialize>(input: I) -> Result<String, anyhow::Error>;
}
impl <'a> Format<'a> for Json {
    fn render<I: Serialize>(input: I) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string(&input)?)
    }
}
impl <'a> Format<'a> for JsonPretty {
    fn render<I: Serialize>(input: I) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string_pretty(&input)?)
    }
}

pub struct TxTraceSerializable<'a> {
    name: Option<&'a str>,
    sql_statements: Vec<SqlStatementCtx<'a>>,
}

impl<'a> TxTraceSerializable<'a> {
    pub fn new(trace: &'a TxLockTracer, show_ddl: bool) -> Self {
        let mut sql_statements = vec![];
        let mut locks = vec![];
        for statement in &trace.statements {
            let ctx = SqlStatementCtx {
                statement_number: sql_statements.len() + 1,
                trace: statement,
                locks_before: locks.clone(),
                show_ddl,
            };
            locks.extend(statement.locks_taken.iter().filter(|lock| show_ddl || lock.mode.dangerous()));
            sql_statements.push(ctx);
        }

        TxTraceSerializable {
            name: trace.name.as_deref(),
            sql_statements,
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct TerseTxTrace<'a> {
    name: Option<&'a str>,
    sql_statements: Vec<TerseSqlStatement<'a>>,
}

impl<'a> From<&'a TxTraceSerializable<'a>> for TerseTxTrace<'a> {
    fn from(value: &'a TxTraceSerializable) -> Self {
        TerseTxTrace {
            name: value.name,
            sql_statements: value
                .sql_statements
                .iter()
                .map(|ctx| ctx.into())
                .collect(),
        }
    }
}
#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct NormalTxTrace<'a> {
    name: Option<&'a str>,
    sql_statements: Vec<NormalSqlStatement<'a>>,
}
impl<'a> From<&'a TxTraceSerializable<'a>> for NormalTxTrace<'a> {
    fn from(value: &'a TxTraceSerializable) -> Self {
        NormalTxTrace {
            name: value.name,
            sql_statements: value
                .sql_statements
                .iter()
                .map(|ctx| ctx.into())
                .collect(),
        }
    }
}
#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct DetailedTxTrace<'a> {
    name: Option<&'a str>,
    sql_statements: Vec<VerboseSqlStatement<'a>>,
}
impl<'a> From<&'a TxTraceSerializable<'a>> for DetailedTxTrace<'a> {
    fn from(value: &'a TxTraceSerializable) -> Self {
        DetailedTxTrace {
            name: value.name,
            sql_statements: value
                .sql_statements
                .iter()
                .map(|ctx| ctx.into())
                .collect(),
        }
    }
}
