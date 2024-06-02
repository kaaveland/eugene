use crate::hint_data;
use crate::hint_data::{HintId, StaticHintData};
use crate::output::output_format::Hint;
use itertools::Itertools;
use std::cmp::Reverse;

use crate::pg_types::contype::Contype;
use crate::pg_types::lock_modes::LockMode;
use crate::pg_types::relkinds::RelKind;
use crate::tracing::tracer::StatementCtx;

type HintFn = fn(&StatementCtx) -> Option<String>;

pub struct HintInfo {
    meta: &'static StaticHintData,
    render_help: HintFn,
}

impl HintId for HintInfo {
    fn id(&self) -> &str {
        self.meta.id
    }
}

impl HintInfo {
    pub fn code(&self) -> &'static str {
        self.meta.id
    }
    pub fn name(&self) -> &'static str {
        self.meta.name
    }
    pub fn condition(&self) -> &'static str {
        self.meta.condition
    }
    pub fn workaround(&self) -> &'static str {
        self.meta.workaround
    }
    pub fn effect(&self) -> &'static str {
        self.meta.effect
    }
}

impl HintInfo {
    pub(crate) fn check(&self, trace: &StatementCtx) -> Option<Hint> {
        (self.render_help)(trace).map(|help| {
            Hint::new(
                self.code(),
                self.name(),
                self.condition(),
                self.effect(),
                self.workaround(),
                help,
            )
        })
    }
}

fn add_new_valid_constraint_help(sql_statement_trace: &StatementCtx) -> Option<String> {
    let constraint = sql_statement_trace.new_constraints().find(|constraint| {
        constraint.valid
            && !matches!(
                constraint.constraint_type,
                Contype::Unique | Contype::Exclusion
            )
    })?;

    let contype = constraint.constraint_type;
    let name = constraint.name.as_str();
    let table = format!("{}.{}", constraint.schema_name, constraint.table_name);

    let help = format!(
        "A new constraint `{name}` of type `{contype}` was added to the table `{table}` as `VALID`. \
                     Constraints that are `NOT VALID` can be made `VALID` by \
                     `ALTER TABLE {table} VALIDATE CONSTRAINT {name}` which takes a lesser lock.",
    );

    Some(help)
}

fn make_column_not_nullable_help(sql_statement_trace: &StatementCtx) -> Option<String> {
    let (id, column) = sql_statement_trace
        .altered_columns()
        .find(|(_, column)| !column.new.nullable && column.old.nullable)?;

    let already_constrained = sql_statement_trace
        .constraints_on(id.oid)
        .filter(|c| c.constraint_type == Contype::Check && c.valid)
        .any(|c| {
            c.expression
                .as_ref()
                .map(|e| {
                    e.to_lowercase().contains(&format!(
                        "{} is not null",
                        column.old.column_name.to_lowercase()
                    ))
                })
                .unwrap_or(false)
        });

    // postgres knows that the column is not null, so it doesn't need to check,
    // making this a safe alter column
    if already_constrained {
        return None;
    }

    let table_name = format!("{}.{}", column.new.schema_name, column.new.table_name);
    let col_name = column.new.column_name.as_str();
    let help = format!(
            "The column `{col_name}` in the table `{table_name}` was changed to `NOT NULL`. \
            If there is a `CHECK ({col_name} IS NOT NULL)` constraint on `{table_name}`, this is safe. \
            Splitting this kind of change into 3 steps can make it safe:\n\n\
            1. Add a `CHECK ({col_name} IS NOT NULL) NOT VALID;` constraint on `{table_name}`.\n\
            2. Validate the constraint in a later transaction, with `ALTER TABLE {table_name} VALIDATE CONSTRAINT ...`.\n\
            3. Make the column `NOT NULL`\n",
        );
    Some(help)
}

fn add_json_column(sql_statement_trace: &StatementCtx) -> Option<String> {
    let column = sql_statement_trace
        .new_columns()
        .find(|column| column.typename == "json")?;

    let help = format!(
            "A new column `{}` of type `json` was added to the table `{}.{}`. The `json` type does not \
             support the equality operator, so this can break `SELECT DISTINCT` queries on the table. \
             Use the `jsonb` type instead.",
            column.column_name,
            column.schema_name,
            column.table_name,
        );
    Some(help)
}

fn running_statement_while_holding_access_exclusive(
    sql_statement_trace: &StatementCtx,
) -> Option<String> {
    let lock = sql_statement_trace
        .locks_at_start()
        .find(|lock| matches!(lock.mode, LockMode::AccessExclusive))?;

    let help = format!(
        "The statement is running while holding an `AccessExclusiveLock` on the {} `{}.{}`, \
                blocking all other transactions from accessing it.",
        lock.target.rel_kind, lock.target.schema, lock.target.object_name,
    );
    Some(help)
}

fn type_change_requires_table_rewrite(sql_statement_trace: &StatementCtx) -> Option<String> {
    let (_, column) = sql_statement_trace
        .altered_columns()
        .find(|(_, column)| column.new.typename != column.old.typename)?;
    let _ = sql_statement_trace
        .rewritten_objects()
        .find(|obj| obj.rel_kind == RelKind::Table)?;

    let help = format!(
            "The column `{}` in the table `{}.{}` was changed from type `{}` to `{}`. This requires \
            an `AccessExclusiveLock` that will block all other transactions from using the table while \
            it is being rewritten.",
            column.new.column_name,
            column.new.schema_name,
            column.new.table_name,
            column.old.typename,
            column.new.typename,
        );
    Some(help)
}

fn new_index_on_existing_table_is_nonconcurrent(
    sql_statement_trace: &StatementCtx,
) -> Option<String> {
    let lock = sql_statement_trace
        .new_locks_taken()
        .find(|lock| matches!(lock.mode, LockMode::Share))?;
    let index = sql_statement_trace
        .new_objects()
        .find(|obj| matches!(obj.rel_kind, RelKind::Index));

    let help = format!(
        "A new index was created on the table `{}.{}`. \
                The index {}was created non-concurrently, which blocks all writes to the table. \
                Use `CREATE INDEX CONCURRENTLY` to avoid blocking writes.",
        lock.target.schema,
        lock.target.object_name,
        index
            .map(|obj| format!("`{}.{}` ", obj.schema, obj.object_name))
            .unwrap_or(String::new())
    );
    Some(help)
}

fn new_unique_constraint_created_index(sql_statement_trace: &StatementCtx) -> Option<String> {
    let constraint = sql_statement_trace
        .new_constraints()
        .find(|constraint| constraint.constraint_type == Contype::Unique)?;
    let index = sql_statement_trace
        .new_objects()
        .find(|obj| matches!(obj.rel_kind, RelKind::Index))?;

    let table = format!("{}.{}", constraint.schema_name, constraint.table_name);
    let name = constraint.name.as_str();
    let index_name = format!("{}.{}", index.schema, index.object_name);

    let help = format!(
                "A new unique constraint `{name}` was added to the table `{table}`. \
                This constraint creates a unique index on the table, and blocks all writes. \
                Consider creating the index concurrently in a separate transaction, then adding \
                the unique constraint by using the index: `ALTER TABLE {table} ADD CONSTRAINT {name} UNIQUE USING INDEX {index_name};`",
            );
    Some(help)
}

fn new_exclusion_constraint_found(sql_statement_trace: &StatementCtx) -> Option<String> {
    let constraint = sql_statement_trace
        .new_constraints()
        .find(|constraint| constraint.constraint_type == Contype::Exclusion)?;

    let help = format!(
        "A new exclusion constraint `{}` was added to the table `{}.{}`. \
                There is no safe way to add an exclusion constraint to an existing table. \
                This constraint creates an index on the table, and blocks all reads and writes.",
        constraint.name, constraint.schema_name, constraint.table_name,
    );
    Some(help)
}

fn took_dangerous_lock_without_timeout(sql_statement_trace: &StatementCtx) -> Option<String> {
    if sql_statement_trace.lock_timeout_millis() > 0 {
        None
    } else {
        let lock = sql_statement_trace
            .new_locks_taken()
            .filter(|lock| lock.mode.dangerous())
            .sorted_by_key(|lock| lock.mode)
            .next_back()?;
        let blocked_queries = lock
            .mode
            .blocked_queries()
            .iter()
            .map(|query| format!("`{query}`"))
            .collect_vec();

        let help = format!(
                    "The statement took `{}` on the {} `{}.{}` without a timeout. It blocks {} while waiting to acquire the lock.",
                    lock.mode, lock.target.rel_kind, lock.target.schema, lock.target.object_name, blocked_queries.join(", "),
                );
        Some(help)
    }
}

fn rewrote_table_or_index(ctx: &StatementCtx) -> Option<String> {
    let rewritten = ctx
        .rewritten_objects()
        .sorted_by_key(|obj| obj.rel_kind) // prioritize tables
        .find(|obj| matches!(obj.rel_kind, RelKind::Index | RelKind::Table))?;
    let lock = ctx
        .locks_at_start()
        .sorted_by_key(|lock| (Reverse(lock.mode), lock.target.rel_kind))
        .find(|lock| lock.mode.dangerous())
        .or_else(|| {
            ctx.new_locks_taken()
                .sorted_by_key(|lock| (Reverse(lock.mode), lock.target.rel_kind))
                .find(|lock| lock.mode.dangerous())
        })?;
    let relkind_rewritten = rewritten.rel_kind.as_str();
    let relkind_locked = lock.target.rel_kind.as_str();
    let blocked_q = lock
        .mode
        .blocked_queries()
        .iter()
        .map(|q| format!("`{}`", q))
        .collect_vec()
        .join(", ");
    let locked_obj = format!("{}.{}", lock.target.schema, lock.target.object_name);
    let rewritten_obj = format!("{}.{}", rewritten.schema_name, rewritten.object_name);
    let mode = lock.mode.to_db_str();
    let help = format!(
        "The {relkind_rewritten} `{rewritten_obj}` was rewritten while holding `{mode}` on the {relkind_locked} `{locked_obj}`\
        . This blocks {blocked_q} while the rewrite is in progress.",
    );
    Some(help)
}

/// All the hints eugene can check statement traces against
pub fn all_hints() -> &'static [HintInfo] {
    HINTS
}

/// Run all hints against a statement trace and return the ones that apply
pub fn run_hints<'a>(trace: &'a StatementCtx) -> impl Iterator<Item = Hint> + 'a {
    HINTS.iter().filter_map(|hint| hint.check(trace))
}
pub const VALIDATE_CONSTRAINT_WITH_LOCK: HintInfo = HintInfo {
    meta: &hint_data::VALIDATE_CONSTRAINT_WITH_LOCK,
    render_help: add_new_valid_constraint_help,
};
pub const MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK: HintInfo = HintInfo {
    meta: &hint_data::MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK,
    render_help: make_column_not_nullable_help,
};
pub const ADD_JSON_COLUMN: HintInfo = HintInfo {
    meta: &hint_data::ADD_JSON_COLUMN,
    render_help: add_json_column,
};
pub const RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE: HintInfo = HintInfo {
    meta: &hint_data::RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE,
    render_help: running_statement_while_holding_access_exclusive,
};
pub const TYPE_CHANGE_REQUIRES_TABLE_REWRITE: HintInfo = HintInfo {
    meta: &hint_data::TYPE_CHANGE_REQUIRES_TABLE_REWRITE,
    render_help: type_change_requires_table_rewrite,
};
pub const NEW_INDEX_ON_EXISTING_TABLE_IS_NONCONCURRENT: HintInfo = HintInfo {
    meta: &hint_data::NEW_INDEX_ON_EXISTING_TABLE_IS_NONCONCURRENT,
    render_help: new_index_on_existing_table_is_nonconcurrent,
};
pub const NEW_UNIQUE_CONSTRAINT_CREATED_INDEX: HintInfo = HintInfo {
    meta: &hint_data::NEW_UNIQUE_CONSTRAINT_CREATED_INDEX,
    render_help: new_unique_constraint_created_index,
};
pub const NEW_EXCLUSION_CONSTRAINT_FOUND: HintInfo = HintInfo {
    meta: &hint_data::NEW_EXCLUSION_CONSTRAINT_FOUND,
    render_help: new_exclusion_constraint_found,
};
pub const TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT: HintInfo = HintInfo {
    meta: &hint_data::TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT,
    render_help: took_dangerous_lock_without_timeout,
};
pub const REWROTE_TABLE_WHILE_HOLDING_DANGEROUS_LOCK: HintInfo = HintInfo {
    meta: &hint_data::REWROTE_TABLE_WHILE_HOLDING_DANGEROUS_LOCK,
    render_help: rewrote_table_or_index,
};

/// All the hints eugene can check statement traces against
const HINTS: &[HintInfo] = &[
    VALIDATE_CONSTRAINT_WITH_LOCK,
    MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK,
    ADD_JSON_COLUMN,
    RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE,
    TYPE_CHANGE_REQUIRES_TABLE_REWRITE,
    NEW_INDEX_ON_EXISTING_TABLE_IS_NONCONCURRENT,
    NEW_UNIQUE_CONSTRAINT_CREATED_INDEX,
    NEW_EXCLUSION_CONSTRAINT_FOUND,
    TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT,
    REWROTE_TABLE_WHILE_HOLDING_DANGEROUS_LOCK,
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[test]
    fn test_no_duplicated_ids() {
        let ids: HashSet<_> = super::all_hints().iter().map(|hint| hint.meta.id).collect();
        assert_eq!(ids.len(), super::all_hints().len());
    }

    #[test]
    fn test_all_are_in_hint_data() {
        super::HINTS.iter().for_each(|hint| {
            assert!(crate::hint_data::ALL
                .iter()
                .any(|data| data.id == hint.meta.id));
        })
    }
}
