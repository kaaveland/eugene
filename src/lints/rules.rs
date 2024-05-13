use pg_query::protobuf::ConstrType;

use crate::hint_data::StaticHintData;
use crate::lints::ast::AlterTableAction;
use crate::lints::{LintedStatement, StatementSummary};
use crate::output::output_format::Hint;

pub struct LintRule {
    meta: &'static StaticHintData,
    check: fn(LintedStatement) -> Option<String>,
}

impl LintRule {
    pub fn id(&self) -> &'static str {
        self.meta.id
    }
    pub fn name(&self) -> &'static str {
        self.meta.name
    }
    pub fn workaround(&self) -> &'static str {
        self.meta.workaround
    }
    pub fn effect(&self) -> &'static str {
        self.meta.effect
    }
    pub fn condition(&self) -> &'static str {
        self.meta.condition
    }
    pub fn check(&self, stmt: LintedStatement) -> Option<Hint> {
        (self.check)(stmt).map(|help| Hint {
            id: self.id().to_string(),
            name: self.name().to_string(),
            effect: self.effect().to_string(),
            workaround: self.workaround().to_string(),
            condition: self.condition().to_string(),
            help,
        })
    }
}

/// Emit a warning if a statement takes a lock that is visible to other transactions without a timeout
pub fn locktimeout_warning(stmt: LintedStatement) -> Option<String> {
    let target = stmt
        .locks_visible_outside_tx()
        .into_iter()
        .find(|(schema, name)| stmt.takes_lock(schema, name));
    match target {
        Some((schema, name)) if !stmt.has_lock_timeout() => Some(format!(
            "Statement takes lock on `{}.{}`, but does not set a lock timeout",
            if schema.is_empty() { "public" } else { schema },
            name
        )),
        _ => None,
    }
}

pub const LOCKTIMEOUT_WARNING: LintRule = LintRule {
    meta: &crate::hint_data::TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT,
    check: locktimeout_warning,
};

fn create_index_nonconcurrently(stmt: LintedStatement) -> Option<String> {
    match stmt.statement {
        StatementSummary::CreateIndex {
            schema,
            idxname,
            target,
            concurrently: false,
            ..
        } if stmt.is_visible(schema, target) => {
            let schema = if schema.is_empty() { "public" } else { schema };
            Some(format!("Statement takes `ShareLock` on `{schema}.{target}`, blocking writes while creating index `{schema}.{idxname}`"))
        }
        _ => None,
    }
}

/// `CREATE INDEX` without `CONCURRENTLY`
pub const CREATE_INDEX_NONCONCURRENTLY: LintRule = LintRule {
    meta: &crate::hint_data::NEW_INDEX_ON_EXISTING_TABLE_IS_NONCONCURRENT,
    check: create_index_nonconcurrently,
};

fn adding_valid_constraint(stmt: LintedStatement) -> Option<String> {
    fn is_valid_constraint(alter_table_cmd: &AlterTableAction) -> bool {
        matches!(
            alter_table_cmd,
            AlterTableAction::AddConstraint {
                valid: true,
                constraint_type: ConstrType::ConstrCheck
                    | ConstrType::ConstrNotnull
                    | ConstrType::ConstrForeign,
                ..
            }
        )
    }
    match stmt.statement {
        StatementSummary::AlterTable {
            schema,
            name,
            actions,
            ..
        } if stmt.is_visible(schema, name) => {
            let schema = if schema.is_empty() { "public" } else { schema };
            let new_constraint = actions.iter().find(|cmd| is_valid_constraint(cmd));
            let table = name;
            if let Some(AlterTableAction::AddConstraint {
                name,
                constraint_type: _,
                ..
            }) = new_constraint
            {
                let name = if name.is_empty() {
                    String::new()
                } else {
                    format!("`{name}` ")
                };
                Some(format!(
                    "Statement takes `AccessExclusiveLock` on `{schema}.{table}`, \
                blocking reads until constraint {name}is validated"
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}
/// Adding a constraint without using `NOT VALID`
pub const ADDING_VALID_CONSTRAINT: LintRule = LintRule {
    meta: &crate::hint_data::VALIDATE_CONSTRAINT_WITH_LOCK,
    check: adding_valid_constraint,
};

fn adding_exclusion_constraint(stmt: LintedStatement) -> Option<String> {
    match stmt.statement {
        StatementSummary::AlterTable {
            schema,
            name,
            actions,
            ..
        } if stmt.is_visible(schema, name) => {
            let new_constraint = actions.iter().find(|cmd| {
                matches!(
                    cmd,
                    AlterTableAction::AddConstraint {
                        constraint_type: ConstrType::ConstrExclusion,
                        ..
                    }
                )
            });
            let table = name;
            if let Some(AlterTableAction::AddConstraint {
                name,
                constraint_type: _,
                ..
            }) = new_constraint
            {
                let schema = if schema.is_empty() { "public" } else { schema };
                Some(format!("Statement takes `AccessExclusiveLock` on `{schema}.{table}`, blocking reads and writes until constraint `{name}` is validated and has created index"))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Adding a new exclusion constraint
pub const ADDING_EXCLUSION_CONSTRAINT: LintRule = LintRule {
    meta: &crate::hint_data::NEW_EXCLUSION_CONSTRAINT_FOUND,
    check: adding_exclusion_constraint,
};

fn add_new_unique_constraint_without_using_index(stmt: LintedStatement) -> Option<String> {
    match stmt.statement {
        StatementSummary::AlterTable {
            schema,
            name,
            actions,
            ..
        } if stmt.is_visible(schema, name) => {
            let schema = if schema.is_empty() { "public" } else { schema };
            let table = name;
            if let Some(AlterTableAction::AddConstraint {
                name,
                use_index: false,
                ..
            }) = actions.iter().find(|cmd| {
                matches!(
                    cmd,
                    AlterTableAction::AddConstraint {
                        constraint_type: ConstrType::ConstrUnique | ConstrType::ConstrPrimary,
                        ..
                    }
                )
            }) {
                Some(format!(
                    "New constraint {name} creates implicit index on `{schema}.{table}`, \
                blocking writes until index is created and validated"
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Letting `add constraint ... unique` create an index using a `ShareLock`
pub const ADD_NEW_UNIQUE_CONSTRAINT_WITHOUT_USING_INDEX: LintRule = LintRule {
    meta: &crate::hint_data::NEW_UNIQUE_CONSTRAINT_CREATED_INDEX,
    check: add_new_unique_constraint_without_using_index,
};

fn run_more_statements_after_taking_access_exclusive(stmt: LintedStatement) -> Option<String> {
    if stmt.holding_access_exclusive() {
        Some("Running more statements after taking `AccessExclusiveLock`".to_string())
    } else {
        None
    }
}

pub const RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE: LintRule = LintRule {
    meta: &crate::hint_data::RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE,
    check: run_more_statements_after_taking_access_exclusive,
};

fn sets_column_to_not_null(stmt: LintedStatement) -> Option<String> {
    match stmt.statement {
        StatementSummary::AlterTable {
            schema,
            name,
            actions,
            ..
        } if stmt.is_visible(schema, name) => {
            let schema = if schema.is_empty() { "public" } else { schema };
            let table = name;
            actions
                .iter()
                .filter_map(|cmd| match cmd {
                    AlterTableAction::SetNotNull { column } => Some(column),
                    _ => None,
                })
                .map(|col_name| {
                    format!(
                        "Statement takes `AccessExclusiveLock` on `{schema}.{table}` by setting \
                     `{col_name}` to `NOT NULL` blocking reads until all rows are validated"
                    )
                })
                .next()
        }
        _ => None,
    }
}

pub const MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK: LintRule = LintRule {
    meta: &crate::hint_data::MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK,
    check: sets_column_to_not_null,
};

fn sets_column_type_to_json(stmt: LintedStatement) -> Option<String> {
    match stmt.statement {
        // TODO: Create table also
        StatementSummary::AlterTable {
            schema,
            name,
            actions,
        } => {
            let added_json = actions
                .iter()
                .filter_map(|cmd| match cmd {
                    AlterTableAction::SetType { type_name, column }
                    | AlterTableAction::AddColumn { type_name, column }
                        if type_name == "json" =>
                    {
                        Some(column)
                    }
                    _ => None,
                })
                .next();
            added_json.map(|column| format!(
                    "Set type of column `{column}` to `json` in `{schema}.{name}`. \
                    The `json` type does not support equality and should not be used, use `jsonb` instead"))
        }
        _ => None,
    }
}

pub const SET_COLUMN_TYPE_TO_JSON: LintRule = LintRule {
    meta: &crate::hint_data::ADD_JSON_COLUMN,
    check: sets_column_type_to_json,
};

fn changes_type_of_column_in_visible_object(stmt: LintedStatement) -> Option<String> {
    match stmt.statement {
        StatementSummary::AlterTable {
            schema,
            name,
            actions,
        } if stmt.is_visible(schema, name) => {
            let changed_column = actions
                .iter()
                .filter_map(|cmd| match cmd {
                    AlterTableAction::SetType { column, type_name } => Some((column, type_name)),
                    _ => None,
                })
                .next();
            changed_column.map(|(column, type_name)| {
                format!(
                    "Changed type of column `{column}` to `{type_name}` in `{schema}.{name}`. \
                    This operation requires a full table rewrite with `AccessExclusiveLock` if `{type_name}` is not binary compatible with \
                    the previous type of `{column}`. Prefer adding a new column with the new type, then dropping/renaming."
                )
            })
        }
        _ => None,
    }
}

pub const CHANGE_COLUMN_TYPE: LintRule = LintRule {
    meta: &crate::hint_data::TYPE_CHANGE_REQUIRES_TABLE_REWRITE,
    check: changes_type_of_column_in_visible_object,
};

const RULES: &[LintRule] = &[
    LOCKTIMEOUT_WARNING,
    CREATE_INDEX_NONCONCURRENTLY,
    ADDING_VALID_CONSTRAINT,
    ADDING_EXCLUSION_CONSTRAINT,
    ADD_NEW_UNIQUE_CONSTRAINT_WITHOUT_USING_INDEX,
    RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE,
    MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK,
    SET_COLUMN_TYPE_TO_JSON,
    CHANGE_COLUMN_TYPE,
];

/// Get all available lint rules
pub fn all_rules() -> impl Iterator<Item = &'static LintRule> {
    RULES.iter()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[test]
    fn test_no_duplicated_ids() {
        let ids: HashSet<_> = super::all_rules().map(|rule| rule.id()).collect();
        assert_eq!(ids.len(), super::all_rules().count());
    }
}