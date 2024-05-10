use crate::output::output_format::Hint;
use itertools::Itertools;

use crate::pg_types::contype::Contype;
use crate::pg_types::lock_modes::LockMode;
use crate::pg_types::relkinds::RelKind;
use crate::tracing::tracer::StatementCtx;

type HintFn = fn(&StatementCtx) -> Option<String>;

pub struct HintInfo {
    pub(crate) code: &'static str,
    pub(crate) name: &'static str,
    pub(crate) condition: &'static str,
    pub(crate) workaround: &'static str,
    pub(crate) effect: &'static str,
    render_help: HintFn,
}

impl HintInfo {
    pub(crate) fn check(&self, trace: &StatementCtx) -> Option<Hint> {
        (self.render_help)(trace).map(|help| {
            Hint::new(
                self.code,
                self.name,
                self.condition,
                self.effect,
                self.workaround,
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
        // TODO: This is not true for all type changes, eg. cidr -> inet is safe
        // TODO: The check is also not sufficient, since varchar(10) -> varchar(20) is safe, but the opposite isn't
        .find(|(_, column)| column.new.typename != column.old.typename)?;
    let help = format!(
            "The column `{}` in the table `{}.{}` was changed from type `{}` to `{}`. This always requires \
            an `AccessExclusiveLock` that will block all other transactions from using the table, and for some \
            type changes, it causes a time-consuming table rewrite.",
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

pub mod ids {
    pub const VALIDATE_CONSTRAINT_WITH_LOCK: &str = "E1";
    pub const MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK: &str = "E2";
    pub const ADD_JSON_COLUMN: &str = "E3";
    pub const RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE: &str = "E4";
    pub const TYPE_CHANGE_REQUIRES_TABLE_REWRITE: &str = "E5";
    pub const NEW_INDEX_ON_EXISTING_TABLE_IS_NONCONCURRENT: &str = "E6";
    pub const NEW_UNIQUE_CONSTRAINT_CREATED_INDEX: &str = "E7";
    pub const NEW_EXCLUSION_CONSTRAINT_FOUND: &str = "E8";
    pub const TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT: &str = "E9";
}
/// All the hints eugene can check statement traces against
pub const HINTS: [HintInfo; 9] = [
    HintInfo {
        name: "Validating table with a new constraint",
        code: ids::VALIDATE_CONSTRAINT_WITH_LOCK,
        condition: "A new constraint was added and it is already `VALID`",
        workaround: "Add the constraint as `NOT VALID` and validate it with `ALTER TABLE ... VALIDATE CONSTRAINT` later",
        effect: "This blocks all table access until all rows are validated",
        render_help: add_new_valid_constraint_help,
    },
    HintInfo {
        name: "Validating table with a new `NOT NULL` column",
        code: ids::MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK,
        condition: "A column was changed from `NULL` to `NOT NULL`",
        workaround: "Add a `CHECK` constraint as `NOT VALID`, validate it later, then make the column `NOT NULL`",
        effect: "This blocks all table access until all rows are validated",
        render_help: make_column_not_nullable_help,
    },
    HintInfo {
        name: "Add a new JSON column",
        code: ids::ADD_JSON_COLUMN,
        condition: "A new column of type `json` was added to a table",
        workaround: "Use the `jsonb` type instead, it supports all use-cases of `json` and is more robust and compact",
        effect: "This breaks `SELECT DISTINCT` queries or other operations that need equality checks on the column",
        render_help: add_json_column,
    },
    HintInfo {
        name: "Running more statements after taking `AccessExclusiveLock`",
        code: ids::RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE,
        condition: "A transaction that holds an `AccessExclusiveLock` started a new statement",
        workaround: "Run this statement in a new transaction",
        effect: "This blocks all access to the table for the duration of this statement",
        render_help: running_statement_while_holding_access_exclusive,
    },
    HintInfo {
        name: "Type change requiring table rewrite",
        code: ids::TYPE_CHANGE_REQUIRES_TABLE_REWRITE,
        condition: "A column was changed to a data type that isn't binary compatible",
        workaround: "Add a new column, update it in batches, and drop the old column",
        effect: "This causes a full table rewrite while holding a lock that prevents all other use of the table",
        render_help: type_change_requires_table_rewrite,
    },
    HintInfo {
        name: "Creating a new index on an existing table",
        code: ids::NEW_INDEX_ON_EXISTING_TABLE_IS_NONCONCURRENT,
        condition: "A new index was created on an existing table without the `CONCURRENTLY` keyword",
        workaround: "Run `CREATE INDEX CONCURRENTLY` instead of `CREATE INDEX`",
        effect: "This blocks all writes to the table while the index is being created",
        render_help: new_index_on_existing_table_is_nonconcurrent,
    },
    HintInfo {
        name: "Creating a new unique constraint",
        code: ids::NEW_UNIQUE_CONSTRAINT_CREATED_INDEX,
        condition: "Found a new unique constraint and a new index",
        workaround: "`CREATE UNIQUE INDEX CONCURRENTLY`, then add the constraint using the index",
        effect: "This blocks all writes to the table while the index is being created and validated",
        render_help: new_unique_constraint_created_index,
    },
    HintInfo {
        name: "Creating a new exclusion constraint",
        code: ids::NEW_EXCLUSION_CONSTRAINT_FOUND,
        condition: "Found a new exclusion constraint",
        workaround: "There is no safe way to add an exclusion constraint to an existing table",
        effect: "This blocks all reads and writes to the table while the constraint index is being created",
        render_help: new_exclusion_constraint_found,
    },
    HintInfo {
        name: "Taking dangerous lock without timeout",
        code: ids::TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT,
        condition: "A lock that would block many common operations was taken without a timeout",
        workaround: "Run `SET LOCAL lock_timeout = '2s';` before the statement and retry the migration if necessary",
        effect: "This can block all other operations on the table indefinitely if any other transaction holds a conflicting lock while `idle in transaction` or `active`",
        render_help: took_dangerous_lock_without_timeout,
    }
];
