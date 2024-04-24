use serde::Serialize;
use crate::pg_types::lock_modes::LockMode;


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
pub struct NormalLockMode<'a> {
    #[serde(flatten)]
    terse: TerseLockMode<'a>,
    used_for: &'a [&'a str],
    conflicts_with: Vec<&'a str>,
}
impl<'a> From<&'a LockMode> for NormalLockMode<'a> {
    fn from(value: &'a LockMode) -> Self {
        NormalLockMode {
            terse: value.into(),
            used_for: value.capabilities(),
            conflicts_with: value
                .conflicts_with()
                .iter()
                .map(|s| s.to_db_str())
                .collect(),
        }
    }
}
#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct DetailedLockMode<'a> {
    #[serde(flatten)]
    normal: NormalLockMode<'a>,
    blocked_queries: Vec<&'a str>,
    blocked_ddl_operations: Vec<&'a str>,
}

impl<'a> From<&'a LockMode> for DetailedLockMode<'a> {
    fn from(value: &'a LockMode) -> Self {
        DetailedLockMode {
            normal: value.into(),
            blocked_queries: value.blocked_queries(),
            blocked_ddl_operations: value.blocked_ddl(),
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct LockModes<L: for<'a> From<&'a LockMode> + Serialize> {
    lock_modes: Vec<L>,
}
