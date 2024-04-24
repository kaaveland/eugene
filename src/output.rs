use serde::Serialize;
use std::fmt::{Display, Formatter};

use crate::output::lock::{DetailedLock, NormalLock, TerseLock};
use crate::output::lock_mode::{DetailedLockMode, NormalLockMode, TerseLockMode};
pub use crate::output::tx_trace::{DetailedTxTrace, NormalTxTrace, TerseTxTrace, TxTraceData};
use crate::pg_types::lock_modes::LockMode;
use crate::pg_types::locks::Lock;

pub mod lock;
pub mod lock_mode;
pub mod sql_statement;
pub mod tx_trace;
/// Specialize this trait to render different levels of detail for different types
pub trait Renderer<'a> {
    /// Select fields by converting a LockMode into this type before rendering
    type LockMode: Serialize + From<&'a LockMode> + Display;
    /// Select fields by converting a Lock into this type before rendering
    type Lock: Serialize + From<&'a Lock> + Display;
    /// Select fields by converting a TxTraceData into this type before rendering
    type TxTrace: Serialize + From<&'a TxTraceData<'a>> + Display;

    /// Render a LockMode into a string using the provided [Format]
    fn lock_mode<F: Format<'a>>(&self, mode: &'a LockMode) -> Result<String, anyhow::Error> {
        let obj: Self::LockMode = mode.into();
        F::render(&obj)
    }
    /// Render a Lock into a string using the provided [Format]
    fn lock<F: Format<'a>>(&self, lock: &'a Lock) -> Result<String, anyhow::Error> {
        let obj: Self::Lock = lock.into();
        F::render(&obj)
    }

    /// Render a TxTraceData into a string using the provided [Format]
    fn trace<F: Format<'a>>(&self, trace: &'a TxTraceData<'a>) -> Result<String, anyhow::Error> {
        let obj: Self::TxTrace = trace.into();
        F::render(&obj)
    }
    /// Render a slice of LockModes into a string using the provided [Format]
    fn lock_modes<F: Format<'a>>(&self, modes: &'a [LockMode]) -> Result<String, anyhow::Error> {
        let obj: Vec<Self::LockMode> = modes.iter().map(|mode| mode.into()).collect();
        let obj = LockModesWrapper { modes: obj };
        F::render(&obj)
    }
}

/// Internally used only to provide Display for Vec<T>
#[derive(Serialize, Debug, Eq, PartialEq)]
struct LockModesWrapper<T> {
    modes: Vec<T>,
}

impl<T: Display> Display for LockModesWrapper<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let vec: Vec<String> = self.modes.iter().map(|mode| format!("{}", mode)).collect();
        write!(f, "{}", vec.join("\n"))
    }
}

/// Terse selects the bare minimum of fields to display in output
pub struct Terse;

impl<'a> Renderer<'a> for Terse {
    type LockMode = TerseLockMode<'a>;
    type Lock = TerseLock<'a>;
    type TxTrace = TerseTxTrace<'a>;
}
/// Normal selects more fields than Terse without being verbose
pub struct Normal;

impl<'a> Renderer<'a> for Normal {
    type LockMode = NormalLockMode<'a>;
    type Lock = NormalLock<'a>;
    type TxTrace = NormalTxTrace<'a>;
}

/// Verbose selects all possible fields
pub struct Detailed;

impl<'a> Renderer<'a> for Detailed {
    type LockMode = DetailedLockMode<'a>;
    type Lock = DetailedLock<'a>;
    type TxTrace = DetailedTxTrace<'a>;
}
/// Output data with [serde_json::to_string_pretty]
pub struct JsonPretty;
/// Format selected fields into a string
pub trait Format<'a> {
    fn render<I: Serialize + Display>(input: I) -> Result<String, anyhow::Error>;
}

impl<'a> Format<'a> for JsonPretty {
    fn render<I: Serialize>(input: I) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string_pretty(&input)?)
    }
}

pub struct PlainText;
impl<'a> Format<'a> for PlainText {
    fn render<I: Display>(input: I) -> Result<String, anyhow::Error> {
        Ok(format!("{}", input))
    }
}
