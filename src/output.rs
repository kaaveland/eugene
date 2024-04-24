use serde::Serialize;

use crate::output::lock::{DetailedLock, NormalLock, TerseLock};
use crate::output::lock_mode::{DetailedLockMode, NormalLockMode, TerseLockMode};
use crate::output::sql_statement::{
    NormalSqlStatement, SqlStatementCtx, TerseSqlStatement, VerboseSqlStatement,
};
pub use crate::output::tx_trace::{DetailedTxTrace, NormalTxTrace, TerseTxTrace, TxTraceData};
use crate::pg_types::lock_modes::LockMode;
use crate::pg_types::locks::Lock;

pub mod lock;
pub mod lock_mode;
pub mod sql_statement;
pub mod tx_trace;
/// Specialize this trait to render different levels of detail for different types
pub trait Renderer<'a> {
    type LockMode: Serialize + From<&'a LockMode>;
    type Lock: Serialize + From<&'a Lock>;
    type SqlStatement: Serialize + From<&'a SqlStatementCtx<'a>>;
    type TxTrace: Serialize + From<&'a TxTraceData<'a>>;

    fn lock_mode<F: Format<'a>>(&self, mode: &'a LockMode) -> Result<String, anyhow::Error> {
        let obj: Self::LockMode = mode.into();
        F::render(&obj)
    }
    fn lock<F: Format<'a>>(&self, lock: &'a Lock) -> Result<String, anyhow::Error> {
        let obj: Self::Lock = lock.into();
        F::render(&obj)
    }
    fn statement<F: Format<'a>>(
        &self,
        statement: &'a SqlStatementCtx<'a>,
    ) -> Result<String, anyhow::Error> {
        let obj: Self::SqlStatement = statement.into();
        F::render(&obj)
    }
    fn trace<F: Format<'a>>(&self, trace: &'a TxTraceData<'a>) -> Result<String, anyhow::Error> {
        let obj: Self::TxTrace = trace.into();
        F::render(&obj)
    }
    fn lock_modes<F: Format<'a>>(&self, modes: &'a [LockMode]) -> Result<String, anyhow::Error> {
        let obj: Vec<Self::LockMode> = modes.iter().map(|mode| mode.into()).collect();
        F::render(&obj)
    }
}

/// Terse selects the bare minimum of fields to display in output
pub struct Terse;

impl<'a> Renderer<'a> for Terse {
    type LockMode = TerseLockMode<'a>;
    type Lock = TerseLock<'a>;
    type SqlStatement = TerseSqlStatement<'a>;
    type TxTrace = TerseTxTrace<'a>;
}
/// Normal selects more fields than Terse without being verbose
pub struct Normal;

impl<'a> Renderer<'a> for Normal {
    type LockMode = NormalLockMode<'a>;
    type Lock = NormalLock<'a>;
    type SqlStatement = NormalSqlStatement<'a>;
    type TxTrace = NormalTxTrace<'a>;
}

/// Verbose selects all possible fields
pub struct Detailed;

impl<'a> Renderer<'a> for Detailed {
    type LockMode = DetailedLockMode<'a>;
    type Lock = DetailedLock<'a>;
    type SqlStatement = VerboseSqlStatement<'a>;
    type TxTrace = DetailedTxTrace<'a>;
}
/// Output data with [serde_json::to_string_pretty]
pub struct JsonPretty;
/// Format selected fields into a string
pub trait Format<'a> {
    fn render<I: Serialize>(input: I) -> Result<String, anyhow::Error>;
}

impl<'a> Format<'a> for JsonPretty {
    fn render<I: Serialize>(input: I) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string_pretty(&input)?)
    }
}
