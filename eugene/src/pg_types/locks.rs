use std::fmt;
use std::fmt::Display;

use postgres::types::Oid;

use crate::pg_types::lock_modes::LockMode;
use crate::pg_types::relkinds::RelKind;

/// A lockable target is a schema object that can be locked, such as a table, or index.
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct LockableTarget {
    pub schema: String,
    pub object_name: String,
    pub rel_kind: RelKind,
    pub oid: Oid,
}

impl LockableTarget {
    pub fn new<S: AsRef<str>>(schema: S, object_name: S, rel_kind: char, oid: Oid) -> Option<Self> {
        Some(Self {
            schema: schema.as_ref().to_string(),
            object_name: object_name.as_ref().to_string(),
            rel_kind: RelKind::from_db_code(rel_kind)?,
            oid,
        })
    }
}

/// A lock targets a target object with a specific mode.
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Lock {
    pub(crate) mode: LockMode,
    pub(crate) target: LockableTarget,
}

impl Lock {
    pub fn target_oid(&self) -> Oid {
        self.target.oid
    }
}

/// Errors that can occur when creating a `Lock`
#[derive(Debug, Eq, PartialEq)]
pub enum InvalidLockError {
    InvalidMode(String),
    InvalidRelKind(char),
}

impl Display for InvalidLockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvalidLockError::InvalidMode(s) => write!(f, "Invalid lock mode: {}", s),
            InvalidLockError::InvalidRelKind(c) => write!(f, "Invalid relation kind: {}", c),
        }
    }
}

impl Lock {
    pub fn new<S: AsRef<str> + Into<String>>(
        schema: S,
        table_name: S,
        mode: S,
        rel_kind: char,
        oid: Oid,
    ) -> Result<Self, InvalidLockError> {
        let mode = LockMode::from_db_str(mode.as_ref())
            .ok_or_else(|| InvalidLockError::InvalidMode(mode.into()))?;
        let target = LockableTarget::new(schema, table_name, rel_kind, oid)
            .ok_or(InvalidLockError::InvalidRelKind(rel_kind))?;
        Ok(Self { mode, target })
    }

    pub fn target(&self) -> &LockableTarget {
        &self.target
    }
    pub fn blocked_queries(&self) -> Vec<&'static str> {
        self.mode.blocked_queries()
    }
    pub fn blocked_ddl(&self) -> Vec<&'static str> {
        self.mode.blocked_ddl()
    }
}
