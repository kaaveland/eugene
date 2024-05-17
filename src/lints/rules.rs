use itertools::Itertools;
use pg_query::protobuf::ConstrType;

use crate::hint_data::{HintId, StaticHintData};
use crate::lints::ast::AlterTableAction;
use crate::lints::{LintContext, StatementSummary};
use crate::output::output_format::Hint;

pub struct LintRule {
    meta: &'static StaticHintData,
    check: fn(LintContext) -> Option<String>,
}

impl HintId for LintRule {
    fn id(&self) -> &str {
        self.meta.id
    }
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
    pub fn check(&self, stmt: LintContext) -> Option<Hint> {
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
pub fn locktimeout_warning(stmt: LintContext) -> Option<String> {
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

fn create_index_nonconcurrently(stmt: LintContext) -> Option<String> {
    match stmt.statement {
        StatementSummary::CreateIndex {
            schema,
            idxname,
            target,
            concurrently: false,
            ..
        } if stmt.is_visible(schema, target) => {
            let schema = if schema.is_empty() { "public" } else { schema };
            Some(format!(
                "Statement takes `ShareLock` on `{schema}.{target}`, blocking \
             writes while creating index `{schema}.{idxname}`"
            ))
        }
        _ => None,
    }
}

/// `CREATE INDEX` without `CONCURRENTLY`
pub const CREATE_INDEX_NONCONCURRENTLY: LintRule = LintRule {
    meta: &crate::hint_data::NEW_INDEX_ON_EXISTING_TABLE_IS_NONCONCURRENT,
    check: create_index_nonconcurrently,
};

fn adding_valid_constraint(stmt: LintContext) -> Option<String> {
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

fn adding_exclusion_constraint(stmt: LintContext) -> Option<String> {
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

fn add_new_unique_constraint_without_using_index(stmt: LintContext) -> Option<String> {
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

fn run_more_statements_after_taking_access_exclusive(stmt: LintContext) -> Option<String> {
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

fn sets_column_to_not_null(stmt: LintContext) -> Option<String> {
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

fn sets_column_type_to_json(stmt: LintContext) -> Option<String> {
    match stmt.statement {
        StatementSummary::AlterTable {
            schema,
            name,
            actions,
        } => {
            let added_json = actions
                .iter()
                .filter_map(|cmd| match cmd {
                    AlterTableAction::SetType { type_name, column }
                    | AlterTableAction::AddColumn {
                        type_name, column, ..
                    } if type_name == "json" => Some(column),
                    _ => None,
                })
                .next();
            added_json.map(|column| format!(
                    "Set type of column `{column}` to `json` in `{schema}.{name}`. \
                    The `json` type does not support equality and should not be used, use `jsonb` instead"))
        }
        StatementSummary::CreateTable { columns, .. } => {
            let added_json = columns
                .iter()
                .filter_map(|column| {
                    if column.type_name == "json" {
                        Some(&column.name)
                    } else {
                        None
                    }
                })
                .next();
            added_json.map(|column| format!(
                    "Created column `{column}` with type `json`. \
                    The `json` type does not support equality and should not be used, use `jsonb` instead"))
        }
        _ => None,
    }
}

pub const SET_COLUMN_TYPE_TO_JSON: LintRule = LintRule {
    meta: &crate::hint_data::ADD_JSON_COLUMN,
    check: sets_column_type_to_json,
};

fn changes_type_of_column_in_visible_object(stmt: LintContext) -> Option<String> {
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

pub fn added_serial_column(stmt: LintContext) -> Option<String> {
    let serials = ["bigserial", "serial"];
    match stmt.statement {
        StatementSummary::AlterTable {
            schema,
            name,
            actions,
        } if stmt.is_visible(schema, name) => {
            let added_serial = actions
                .iter()
                .filter_map(|cmd| match cmd {
                    AlterTableAction::AddColumn {
                        type_name,
                        column,
                        stored_generated: generated_always,
                    } if *generated_always || serials.contains(&type_name.as_str()) => Some(column),
                    _ => None,
                })
                .next();
            added_serial.map(|column| {
                format!(
                    "Added column `{column}` with type that will force table rewrite  in `{schema}.{name}`. \
                    `serial` types and `GENERATED ALWAYS as ... STORED` columns require a full table rewrite with `AccessExclusiveLock`"
                )
            })
        }
        _ => None,
    }
}

pub const ADD_SERIAL_COLUMN: LintRule = LintRule {
    meta: &crate::hint_data::ADDED_SERIAL_OR_STORED_GENERATED_COLUMN,
    check: added_serial_column,
};

pub fn multiple_alter_table_with_same_target(ctx: LintContext) -> Option<String> {
    match ctx.statement {
        StatementSummary::AlterTable { schema, name, .. }
            if ctx.is_visible(schema, name) && ctx.has_altered_table(schema, name) =>
        {
            let schema = if schema.is_empty() { "public" } else { schema };
            Some(format!(
                "Multiple `ALTER TABLE` statements on `{schema}.{name}`. \
                    Combine them into a single statement to avoid scanning the table multiple times."
            ))
        }
        _ => None,
    }
}

pub const MULTIPLE_ALTER_TABLES_WHERE_ONE_WILL_DO: LintRule = LintRule {
    meta: &crate::hint_data::MULTIPLE_ALTER_TABLES_WHERE_ONE_WILL_DO,
    check: multiple_alter_table_with_same_target,
};

pub fn creating_enum(ctx: LintContext) -> Option<String> {
    match ctx.statement {
        StatementSummary::CreateEnum { name, .. } => Some(format!(
            "Created enum `{name}`. \
                Enumerated types are not recommended for use in new applications. \
                Consider using a foreign key to a lookup table instead."
        )),
        _ => None,
    }
}
pub const CREATING_ENUM: LintRule = LintRule {
    meta: &crate::hint_data::CREATING_ENUM,
    check: creating_enum,
};

fn add_primary_key_constraint_using_index(ctx: LintContext) -> Option<String> {
    match ctx.statement {
        StatementSummary::AlterTable {
            schema,
            name,
            actions,
            ..
        } if ctx.is_visible(schema, name) => {
            let schema = if schema.is_empty() { "public" } else { schema };
            let table = name;
            actions.iter().filter_map(|cmd| {
                if let AlterTableAction::AddConstraint {
                    constraint_type: ConstrType::ConstrPrimary,
                    use_index: true,
                    ..
                } = cmd
                {
                    Some(format!(
                        "New primary key constraint using index on `{schema}.{table}`, \
                    may cause postgres to `SET NOT NULL` on columns in the index. \
                    This lint may be a false positive if the columns are already `NOT NULL`, ignore it \
                    by commenting the statement with -- eugene: ignore: {}", ADD_PRIMARY_KEY_USING_INDEX.id()
                    ))
                } else {
                    None
                }
            }).next()
        }
        _ => None,
    }
}
pub const ADD_PRIMARY_KEY_USING_INDEX: LintRule = LintRule {
    meta: &crate::hint_data::ADD_PRIMARY_KEY_USING_INDEX,
    check: add_primary_key_constraint_using_index,
};
const RULES: &[LintRule] = &[
    ADDING_VALID_CONSTRAINT,
    MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK,
    SET_COLUMN_TYPE_TO_JSON,
    RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE,
    CHANGE_COLUMN_TYPE,
    CREATE_INDEX_NONCONCURRENTLY,
    ADD_NEW_UNIQUE_CONSTRAINT_WITHOUT_USING_INDEX,
    ADDING_EXCLUSION_CONSTRAINT,
    LOCKTIMEOUT_WARNING,
    ADD_SERIAL_COLUMN,
    MULTIPLE_ALTER_TABLES_WHERE_ONE_WILL_DO,
    CREATING_ENUM,
    ADD_PRIMARY_KEY_USING_INDEX,
];

/// Get all available lint rules
pub fn all_rules() -> impl Iterator<Item = &'static LintRule> {
    RULES.iter().sorted_by_key(|rule| rule.id())
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
