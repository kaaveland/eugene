use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::hint_data::{hint_url, HintId, StaticHintData};
use crate::hints::HintInfo;
use crate::pg_types::locks::LockableTarget;
use crate::tracing::queries::ColumnMetadata;

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct GenericHint {
    pub id: String,
    pub name: String,
    pub condition: String,
    pub effect: String,
    pub workaround: String,
    pub has_lint: bool,
    pub has_trace: bool,
    pub url: String,
}

impl From<&HintInfo> for GenericHint {
    fn from(value: &HintInfo) -> Self {
        GenericHint {
            id: value.code().to_string(),
            name: value.name().to_string(),
            condition: value.condition().to_string(),
            effect: value.effect().to_string(),
            workaround: value.workaround().to_string(),
            has_lint: crate::lints::rules::all_rules().any(|rule| rule.id() == value.code()),
            has_trace: crate::hints::all_hints()
                .iter()
                .any(|hint| hint.code() == value.code()),
            url: value.url(),
        }
    }
}

impl From<&StaticHintData> for GenericHint {
    fn from(value: &StaticHintData) -> Self {
        GenericHint {
            id: value.id.to_string(),
            name: value.name.to_string(),
            condition: value.condition.to_string(),
            effect: value.effect.to_string(),
            workaround: value.workaround.to_string(),
            has_lint: crate::lints::rules::all_rules().any(|rule| rule.id() == value.id),
            has_trace: crate::hints::all_hints()
                .iter()
                .any(|hint| hint.code() == value.id),
            url: value.url(),
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
    pub lock_duration_millis: u64,
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

impl From<&crate::tracing::queries::Constraint> for Constraint {
    fn from(constraint: &crate::tracing::queries::Constraint) -> Self {
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
    pub line_number: usize,
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
    pub triggered_rules: Vec<Hint>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct FullTraceData {
    pub name: Option<String>,
    #[serde(with = "datefmt")]
    pub start_time: DateTime<Utc>,
    pub total_duration_millis: u64,
    pub all_locks_acquired: Vec<TracedLock>,
    pub statements: Vec<FullSqlStatementLockTrace>,
    pub skip_summary: bool,
    pub dangerous_locks_count: usize,
    pub passed_all_checks: bool,
}

mod datefmt {
    use chrono::{DateTime, Utc};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
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
    pub url: String,
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
            url: hint_url(code),
        }
    }
}

#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub struct LintedStatement {
    pub statement_number: usize,
    pub line_number: usize,
    pub sql: String,
    pub triggered_rules: Vec<Hint>,
}

#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub struct LintReport {
    pub name: Option<String>,
    pub statements: Vec<LintedStatement>,
    pub passed_all_checks: bool,
}
