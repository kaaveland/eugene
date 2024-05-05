use crate::output::FullSqlStatementLockTrace;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Hint {
    pub code: &'static str,
    pub name: &'static str,
    pub help: String,
}

fn add_new_valid_constraint(sql_statement_trace: &FullSqlStatementLockTrace) -> Option<Hint> {
    let cons = sql_statement_trace
        .new_constraints
        .iter()
        .find(|constraint| constraint.valid);

    cons.map(|constraint| Hint {
        name: "Validating table with a new constraint",
        code: "validate_constraint_with_lock",
        help: format!(
            "A new constraint `{}` was added to the table `{}`. \
            The constraint is of type `{}` and is valid. The statement blocks until all rows \
            in the table are validated for the constraint. It is safer to add constraints as `NOT VALID` \
            and validate them later, to avoid holding dangerous locks for a long time. Constraints that are `NOT VALID` \
            affect all new inserts and updates, but not existing data. Adding the constraint initially as `NOT VALID`, then \
            validating with `ALTER TABLE ... VALIDATE CONSTRAINT ...` in a later transaction minimizes time spent holding \
             dangerous locks.",
            constraint.name,
            constraint.table_name,
            constraint.constraint_type,
        ),
    })
}

fn make_column_not_nullable(sql_statement_trace: &FullSqlStatementLockTrace) -> Option<Hint> {
    let columns = sql_statement_trace
        .altered_columns
        .iter()
        .find(|column| !column.new.nullable && column.old.nullable);

    columns.map(|column| Hint {
        name: "Validating table with a new `NOT NULL` column",
        code: "make_column_not_nullable_with_lock",
        help: format!(
            "The column `{}` in the table `{}.{}` was changed to `NOT NULL`. \
            The statement blocks until all rows in the table are validated to be `NOT NULL`, unless a \
             `CHECK ({} IS NOT NULL)` constraint exists, in which case it is safe. \
            Splitting this kind of change into 3 steps can make it safer:\n\n \
            1. Add a `CHECK ({} IS NOT NULL) NOT VALID;` constraint.\n\
            2. Validate the constraint in a later transaction, with `ALTER TABLE ... VALIDATE CONSTRAINT`.\n\
            3. Make the column `NOT NULL`\n",
            column.new.column_name,
            column.new.schema_name,
            column.new.table_name,
            column.new.column_name,
            column.new.column_name,
        ),
    })
}

fn add_json_column(sql_statement_trace: &FullSqlStatementLockTrace) -> Option<Hint> {
    let columns = sql_statement_trace
        .new_columns
        .iter()
        .find(|column| column.data_type == "json");

    columns.map(|column| Hint {
        name: "Validating table with a new JSON column",
        code: "add_json_column",
        help: format!(
            "A new column `{}` of type `json` was added to the table `{}.{}`. The `json` type does not \
             support the equality operator, so this can break `SELECT DISTINCT` queries on the table. \
             Use the `jsonb` type instead.",
            column.column_name,
            column.schema_name,
            column.table_name,
        ),
    })
}

fn running_statement_while_holding_access_exclusive(
    sql_statement_trace: &FullSqlStatementLockTrace,
) -> Option<Hint> {
    sql_statement_trace
        .locks_at_start
        .iter()
        .find(|lock| lock.mode == "AccessExclusiveLock")
        .map(|lock| Hint {
            name: "Running more statements after taking `AccessExclusiveLock`",
            code: "holding_access_exclusive",
            help: format!(
                "The statement is running while holding an `AccessExclusiveLock` on the {} `{}.{}`, \
                blocking all other transactions from accessing it. \
                Once holding `AccessExclusiveLock` we should immediately commit the transaction. \
                Any extra steps necessary are better done in a separate transaction.",
                lock.relkind, lock.schema, lock.object_name,
            ),
        })
}

fn type_change_requires_table_rewrite(
    sql_statement_trace: &FullSqlStatementLockTrace,
) -> Option<Hint> {
    sql_statement_trace
        .altered_columns
        .iter()
        // TODO: This is not true for all type changes, eg. cidr -> inet is safe
        // TODO: The check is also not sufficient, since varchar(10) -> varchar(20) is safe, but the opposite isn't
        .find(|column| column.new.data_type != column.old.data_type)
        .map(|column| Hint {
            name: "Type change requiring table rewrite",
            code: "type_change_requires_table_rewrite",
            help: format!(
                "The column `{}` in the table `{}.{}` was changed from type `{}` to `{}`. \
                This change requires a full table rewrite, which can be slow on large tables. \
                Consider adding a new column with the new type, updating it in batches, and then dropping the old column.",
                column.new.column_name,
                column.new.schema_name,
                column.new.table_name,
                column.old.data_type,
                column.new.data_type,
            ),
        })
}

pub type HintCheck = Box<dyn Fn(&FullSqlStatementLockTrace) -> Option<Hint>>;

/// Returns all known hints that can be checked against a `FullSqlStatementLockTrace`.
pub fn checks() -> Vec<HintCheck> {
    vec![
        Box::new(add_new_valid_constraint),
        Box::new(make_column_not_nullable),
        Box::new(add_json_column),
        Box::new(running_statement_while_holding_access_exclusive),
        Box::new(type_change_requires_table_rewrite),
    ]
}
