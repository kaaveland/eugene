use chrono::{DateTime, Local};
use serde::Serialize;

use crate::hints::HintInfo;
use crate::pg_types::locks::{Lock, LockableTarget};
use crate::tracing::tracer::ColumnMetadata;

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct GenericHint {
    pub id: String,
    pub name: String,
    pub condition: String,
    pub effect: String,
    pub workaround: String,
}

impl From<&HintInfo> for GenericHint {
    fn from(value: &HintInfo) -> Self {
        GenericHint {
            id: value.code.to_string(),
            name: value.name.to_string(),
            condition: value.condition.to_string(),
            effect: value.effect.to_string(),
            workaround: value.workaround.to_string(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct DbObject {
    pub schema: String,
    pub object_name: String,
    pub relkind: &'static str,
    pub oid: u32,
}

impl From<&LockableTarget> for DbObject {
    fn from(value: &LockableTarget) -> Self {
        DbObject {
            schema: value.schema.to_string(),
            object_name: value.object_name.to_string(),
            relkind: value.rel_kind.as_str(),
            oid: value.oid,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct TracedLock {
    pub schema: String,
    pub object_name: String,
    pub mode: String,
    pub relkind: &'static str,
    pub oid: u32,
    pub maybe_dangerous: bool,
    pub blocked_queries: Vec<&'static str>,
    pub blocked_ddl: Vec<&'static str>,
}

impl From<&Lock> for TracedLock {
    fn from(lock: &Lock) -> Self {
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
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct Column {
    pub schema_name: String,
    pub table_name: String,
    pub column_name: String,
    pub data_type: String,
    pub nullable: bool,
}

impl From<&ColumnMetadata> for Column {
    fn from(meta: &ColumnMetadata) -> Self {
        Column {
            schema_name: meta.schema_name.clone(),
            table_name: meta.table_name.clone(),
            column_name: meta.column_name.clone(),
            data_type: meta.typename.clone(),
            nullable: meta.nullable,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct ModifiedColumn {
    pub old: Column,
    pub new: Column,
}

impl From<&crate::tracing::tracer::ModifiedColumn> for ModifiedColumn {
    fn from(meta: &crate::tracing::tracer::ModifiedColumn) -> Self {
        ModifiedColumn {
            old: Column::from(&meta.old),
            new: Column::from(&meta.new),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct Constraint {
    pub schema_name: String,
    pub table_name: String,
    pub name: String,
    pub constraint_name: String,
    pub constraint_type: &'static str,
    pub valid: bool,
    pub definition: Option<String>,
}

impl From<&crate::tracing::tracer::Constraint> for Constraint {
    fn from(constraint: &crate::tracing::tracer::Constraint) -> Self {
        Constraint {
            schema_name: constraint.schema_name.clone(),
            table_name: constraint.table_name.clone(),
            name: constraint.name.clone(),
            constraint_name: constraint.name.clone(),
            constraint_type: constraint.constraint_type.to_display(),
            valid: constraint.valid,
            definition: constraint.expression.clone(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct ModifiedConstraint {
    pub old: Constraint,
    pub new: Constraint,
}

impl From<&crate::tracing::tracer::ModifiedConstraint> for ModifiedConstraint {
    fn from(meta: &crate::tracing::tracer::ModifiedConstraint) -> Self {
        ModifiedConstraint {
            old: Constraint::from(&meta.old),
            new: Constraint::from(&meta.new),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct FullSqlStatementLockTrace {
    pub statement_number_in_transaction: usize,
    pub sql: String,
    pub duration_millis: u64,
    pub start_time_millis: u64,
    pub locks_at_start: Vec<TracedLock>,
    pub new_locks_taken: Vec<TracedLock>,
    pub new_columns: Vec<Column>,
    pub altered_columns: Vec<ModifiedColumn>,
    pub new_constraints: Vec<Constraint>,
    pub altered_constraints: Vec<ModifiedConstraint>,
    pub new_objects: Vec<DbObject>,
    pub lock_timeout_millis: u64,
    pub triggered_hints: Vec<Hint>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct FullTraceData {
    pub name: Option<String>,
    #[serde(with = "datefmt")]
    pub start_time: DateTime<Local>,
    pub total_duration_millis: u64,
    pub all_locks_acquired: Vec<TracedLock>,
    pub statements: Vec<FullSqlStatementLockTrace>,
    #[serde(skip)]
    pub(crate) skip_summary: bool,
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

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct Hint {
    pub id: String,
    pub name: String,
    pub condition: String,
    pub effect: String,
    pub workaround: String,
    pub help: String,
}

impl Hint {
    pub fn new(
        code: &str,
        name: &str,
        condition: &str,
        effect: &str,
        workaround: &str,
        help: String,
    ) -> Self {
        Hint {
            id: code.to_string(),
            name: name.to_string(),
            condition: condition.to_string(),
            effect: effect.to_string(),
            workaround: workaround.to_string(),
            help: help.to_string(),
        }
    }
}
