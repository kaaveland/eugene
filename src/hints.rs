use itertools::Itertools;
use serde_json::map;
use std::iter;

use crate::output::FullSqlStatementLockTrace;
use crate::pg_types::lock_modes;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Hint {
    pub code: &'static str,
    pub name: &'static str,
    pub help: String,
    pub condition: &'static str,
    pub workaround: &'static str,
    pub effect: &'static str,
}

type HintFn = fn(&FullSqlStatementLockTrace) -> Option<String>;

pub struct HintInfo {
    pub(crate) code: &'static str,
    pub(crate) name: &'static str,
    pub(crate) condition: &'static str,
    pub(crate) workaround: &'static str,
    pub(crate) effect: &'static str,
    render_help: HintFn,
}

impl HintInfo {
    pub fn check(&self, trace: &FullSqlStatementLockTrace) -> Option<Hint> {
        (self.render_help)(trace).map(|help| Hint {
            code: self.code,
            name: self.name,
            help,
            condition: self.condition,
            workaround: self.workaround,
            effect: self.effect,
        })
    }
}

fn add_new_valid_constraint_help(
    sql_statement_trace: &FullSqlStatementLockTrace,
) -> Option<String> {
    let constraint = sql_statement_trace
        .new_constraints
        .iter()
        .find(|constraint| {
            constraint.valid
                && constraint.constraint_type != "UNIQUE"
                && constraint.constraint_type != "EXCLUSION"
        })?;

    let help = format!(
        "A new constraint `{}` of type `{}` was added to the table `{}` as `VALID`. \
                     Constraints that are `NOT VALID` can be made `VALID` by \
                     `ALTER TABLE {}.{} VALIDATE CONSTRAINT {}` which takes a lesser lock.",
        constraint.name,
        constraint.constraint_type,
        constraint.table_name,
        constraint.schema_name,
        constraint.table_name,
        constraint.name
    );

    Some(help)
}

fn make_column_not_nullable_help(
    sql_statement_trace: &FullSqlStatementLockTrace,
) -> Option<String> {
    let column = sql_statement_trace
        .altered_columns
        .iter()
        .find(|column| !column.new.nullable && column.old.nullable)?;

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

fn add_json_column(sql_statement_trace: &FullSqlStatementLockTrace) -> Option<String> {
    let column = sql_statement_trace
        .new_columns
        .iter()
        .find(|column| column.data_type == "json")?;

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
    sql_statement_trace: &FullSqlStatementLockTrace,
) -> Option<String> {
    let lock = sql_statement_trace
        .locks_at_start
        .iter()
        .find(|lock| lock.mode == "AccessExclusiveLock")?;

    let help = format!(
        "The statement is running while holding an `AccessExclusiveLock` on the {} `{}.{}`, \
                blocking all other transactions from accessing it.",
        lock.relkind, lock.schema, lock.object_name,
    );
    Some(help)
}

fn type_change_requires_table_rewrite(
    sql_statement_trace: &FullSqlStatementLockTrace,
) -> Option<String> {
    let column = sql_statement_trace
        .altered_columns
        .iter()
        // TODO: This is not true for all type changes, eg. cidr -> inet is safe
        // TODO: The check is also not sufficient, since varchar(10) -> varchar(20) is safe, but the opposite isn't
        .find(|column| column.new.data_type != column.old.data_type)?;
    let help = format!(
            "The column `{}` in the table `{}.{}` was changed from type `{}` to `{}`. This always requires \
            an `AccessExclusiveLock` that will block all other transactions from using the table, and for some \
            type changes, it causes a time-consuming table rewrite.",
            column.new.column_name,
            column.new.schema_name,
            column.new.table_name,
            column.old.data_type,
            column.new.data_type,
        );
    Some(help)
}

fn new_index_on_existing_table_is_nonconcurrent(
    sql_statement_trace: &FullSqlStatementLockTrace,
) -> Option<String> {
    let (lock, index) = sql_statement_trace
        .new_locks_taken
        .iter()
        .find(|lock| lock.mode == "ShareLock")
        .map(|lock| {
            (
                lock,
                sql_statement_trace
                    .new_objects
                    .iter()
                    .find(|obj| obj.relkind == "Index"),
            )
        })?;
    let help = format!(
        "A new index was created on the table `{}.{}`. \
                The index {}was created non-concurrently, which blocks all writes to the table. \
                Use `CREATE INDEX CONCURRENTLY` to avoid blocking writes.",
        lock.schema,
        lock.object_name,
        index
            .map(|obj| format!("`{}.{}` ", obj.schema, obj.object_name))
            .unwrap_or(String::new())
    );
    Some(help)
}

fn new_unique_constraint_created_index(
    sql_statement_trace: &FullSqlStatementLockTrace,
) -> Option<String> {
    let (constraint, index) = sql_statement_trace
        .new_constraints
        .iter()
        .find(|constraint| constraint.constraint_type == "UNIQUE")
        .and_then(|constraint| {
            sql_statement_trace
                .new_objects
                .iter()
                .find(|obj| obj.relkind == "Index")
                .map(|index| (constraint, index))
        })?;

    let help = format!(
                "A new unique constraint `{}` was added to the table `{}.{}`. \
                This constraint creates a unique index on the table, and blocks all writes. \
                Consider creating the index concurrently in a separate transaction, then adding \
                the unqiue constraint by using the index: `ALTER TABLE {}.{} ADD CONSTRAINT {} UNIQUE USING INDEX {}.{};`",
                constraint.name,
                constraint.schema_name,
                constraint.table_name,
                constraint.schema_name,
                constraint.table_name,
                constraint.name,
                index.schema,
                index.object_name,
            );
    Some(help)
}

fn new_exclusion_constraint_found(
    sql_statement_trace: &FullSqlStatementLockTrace,
) -> Option<String> {
    sql_statement_trace
        .new_constraints
        .iter()
        .find(|constraint| constraint.constraint_type == "EXCLUSION")
        .map(|constraint| {
            format!(
                "A new exclusion constraint `{}` was added to the table `{}.{}`. \
                There is no safe way to add an exclusion constraint to an existing table. \
                This constraint creates an index on the table, and blocks all reads and writes.",
                constraint.name, constraint.schema_name, constraint.table_name,
            )
        })
}

fn took_dangerous_lock_without_timeout(
    sql_statement_trace: &FullSqlStatementLockTrace,
) -> Option<String> {
    if sql_statement_trace.lock_timeout_millis > 0 {
        None
    } else {
        let lock = sql_statement_trace.new_locks_taken.iter().find(|lock| {
            lock_modes::LOCK_MODES
                .iter()
                .any(|mode| mode.to_db_str() == lock.mode && mode.dangerous())
        })?;
        let blocked_queries = lock_modes::LockMode::from_db_str(lock.mode.as_str())?
            .blocked_queries()
            .iter()
            .map(|query| format!("`{query}`"))
            .collect_vec();

        let help = format!(
                    "The statement took `{}` on the {} `{}.{}` without a timeout. It blocks {} while waiting to acquire the lock.",
                    lock.mode, lock.relkind, lock.schema, lock.object_name, blocked_queries.join(", "),
                );
        Some(help)
    }
}

/// All the hints eugene can check statement traces against
pub const HINTS: [HintInfo; 9] = [
    HintInfo {
        name: "Validating table with a new constraint",
        code: "validate_constraint_with_lock",
        condition: "A new constraint was added and it is already `VALID`",
        workaround: "Add the constraint as `NOT VALID` and validate it with `ALTER TABLE ... VALIDATE CONSTRAINT` later",
        effect: "This blocks all table access until all rows are validated",
        render_help: add_new_valid_constraint_help,
    },
    HintInfo {
        name: "Validating table with a new `NOT NULL` column",
        code: "make_column_not_nullable_with_lock",
        condition: "A column was changed from `NULL` to `NOT NULL`",
        workaround: "Add a `CHECK` constraint as `NOT VALID`, validate it later, then make the column `NOT NULL`",
        effect: "This blocks all table access until all rows are validated",
        render_help: make_column_not_nullable_help,
    },
    HintInfo {
        name: "Add a new JSON column",
        code: "add_json_column",
        condition: "A new column of type `json` was added to a table",
        workaround: "Use the `jsonb` type instead, it supports all use-cases of `json` and is more robust and compact",
        effect: "This breaks `SELECT DISTINCT` queries or other operations that need equality checks on the column",
        render_help: add_json_column,
    },
    HintInfo {
        name: "Running more statements after taking `AccessExclusiveLock`",
        code: "holding_access_exclusive",
        condition: "A transaction that holds an `AccessExclusiveLock` started a new statement",
        workaround: "Run this statement in a new transaction",
        effect: "This blocks all access to the table for the duration of this statement",
        render_help: running_statement_while_holding_access_exclusive,
    },
    HintInfo {
        name: "Type change requiring table rewrite",
        code: "type_change_requires_table_rewrite",
        condition: "A column was changed to a data type that isn't binary compatible",
        workaround: "Add a new column, update it in batches, and drop the old column",
        effect: "This causes a full table rewrite while holding a lock that prevents all other use of the table",
        render_help: type_change_requires_table_rewrite,
    },
    HintInfo {
        name: "Creating a new index on an existing table",
        code: "new_index_on_existing_table_is_nonconcurrent",
        condition: "A new index was created on an existing table without the `CONCURRENTLY` keyword",
        workaround: "Run `CREATE INDEX CONCURRENTLY` instead of `CREATE INDEX`",
        effect: "This blocks all writes to the table while the index is being created",
        render_help: new_index_on_existing_table_is_nonconcurrent,
    },
    HintInfo {
        name: "Creating a new unique constraint",
        code: "new_unique_constraint_created_index",
        condition: "Found a new unique constraint and a new index",
        workaround: "`CREATE UNIQUE INDEX CONCURRENTLY`, then add the constraint using the index",
        effect: "This blocks all writes to the table while the index is being created and validated",
        render_help: new_unique_constraint_created_index,
    },
    HintInfo {
        name: "Creating a new exclusion constraint",
        code: "new_exclusion_constraint_created",
        condition: "Found a new exclusion constraint",
        workaround: "There is no safe way to add an exclusion constraint to an existing table",
        effect: "This blocks all reads and writes to the table while the constraint index is being created",
        render_help: new_exclusion_constraint_found,
    },
    HintInfo {
        name: "Taking dangerous lock without timeout",
        code: "dangerous_lock_without_timeout",
        condition: "A lock that would block many common operations was taken without a timeout",
        workaround: "Run `SET lock_timeout = '2s';` before the statement and retry the migration if necessary",
        effect: "This can block all other operations on the table indefinitely if any other transaction holds a conflicting lock while `idle in transaction` or `active`",
        render_help: took_dangerous_lock_without_timeout,
    }
];
