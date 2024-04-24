use crate::pg_types::lock_modes::LockMode;
use serde::Serialize;
use std::fmt;
use std::fmt::Display;

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

impl Display for TerseLockMode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "mode: {}", self.lock_mode)
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
impl Display for NormalLockMode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}, ", self.terse)?;
        writeln!(f, "used for: {:?}", self.used_for)?;
        write!(f, "conflicts with: {:?}", self.conflicts_with)
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct DetailedLockMode<'a> {
    #[serde(flatten)]
    normal: NormalLockMode<'a>,
    blocked_queries: Vec<&'a str>,
    blocked_ddl_operations: Vec<&'a str>,
}

impl Display for DetailedLockMode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.normal)?;
        writeln!(f, "  blocked queries: {:?}", self.blocked_queries)?;
        write!(
            f,
            "  blocked ddl operations: {:?}",
            self.blocked_ddl_operations
        )
    }
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

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::pg_types::lock_modes::LockMode;
    #[test]
    fn test_to_document_display_output_format_for_lock_modes() {
        let lock_mode = LockMode::AccessShare;
        let terse = TerseLockMode::from(&lock_mode);
        assert_eq!(format!("{}", terse), "mode: AccessShareLock");
        let normal = NormalLockMode::from(&lock_mode);
        assert_eq!(format!("{}", normal), "mode: AccessShareLock, used for: [\"SELECT\"]\nconflicts with: [\"AccessExclusiveLock\"]");
        let detailed = DetailedLockMode::from(&lock_mode);
        assert_eq!(format!("{}", detailed), "mode: AccessShareLock, used for: [\"SELECT\"]\nconflicts with: [\"AccessExclusiveLock\"]
  blocked queries: []\n  blocked ddl operations: [\"ALTER TABLE\", \"DROP TABLE\", \"TRUNCATE\", \"REINDEX\", \"CLUSTER\", \"VACUUM FULL\", \"REFRESH MATERIALIZED VIEW\"]");
    }
}
