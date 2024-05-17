pub trait HintId {
    fn id(&self) -> &str;
}

pub struct StaticHintData {
    pub id: &'static str,
    pub name: &'static str,
    pub condition: &'static str,
    pub effect: &'static str,
    pub workaround: &'static str,
}

impl HintId for StaticHintData {
    fn id(&self) -> &str {
        self.id
    }
}

pub const VALIDATE_CONSTRAINT_WITH_LOCK: StaticHintData = StaticHintData {
    id: "E1",
    name: "Validating table with a new constraint",
    condition: "A new constraint was added and it is already `VALID`",
    effect: "This blocks all table access until all rows are validated",
    workaround: "Add the constraint as `NOT VALID` and validate it with `ALTER TABLE ... VALIDATE CONSTRAINT` later",
};
pub const MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK: StaticHintData = StaticHintData {
    id: "E2",
    name: "Validating table with a new `NOT NULL` column",
    condition: "A column was changed from `NULL` to `NOT NULL`",
    workaround: "Add a `CHECK` constraint as `NOT VALID`, validate it later, then make the column `NOT NULL`",
    effect: "This blocks all table access until all rows are validated",
};
pub const ADD_JSON_COLUMN: StaticHintData = StaticHintData {
    id: "E3",
    name: "Add a new JSON column",
    condition: "A new column of type `json` was added to a table",
    workaround: "Use the `jsonb` type instead, it supports all use-cases of `json` and is more robust and compact",
    effect: "This breaks `SELECT DISTINCT` queries or other operations that need equality checks on the column",
};
pub const RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE: StaticHintData = StaticHintData {
    id: "E4",
    name: "Running more statements after taking `AccessExclusiveLock`",
    condition: "A transaction that holds an `AccessExclusiveLock` started a new statement",
    workaround: "Run this statement in a new transaction",
    effect: "This blocks all access to the table for the duration of this statement",
};
pub const TYPE_CHANGE_REQUIRES_TABLE_REWRITE: StaticHintData = StaticHintData {
    id: "E5",
    name: "Type change requiring table rewrite",
    condition: "A column was changed to a data type that isn't binary compatible",
    workaround: "Add a new column, update it in batches, and drop the old column",
    effect: "This causes a full table rewrite while holding a lock that prevents all other use of the table",
};
pub const NEW_INDEX_ON_EXISTING_TABLE_IS_NONCONCURRENT: StaticHintData = StaticHintData {
    id: "E6",
    name: "Creating a new index on an existing table",
    condition: "A new index was created on an existing table without the `CONCURRENTLY` keyword",
    workaround: "Run `CREATE INDEX CONCURRENTLY` instead of `CREATE INDEX`",
    effect: "This blocks all writes to the table while the index is being created",
};
pub const NEW_UNIQUE_CONSTRAINT_CREATED_INDEX: StaticHintData = StaticHintData {
    id: "E7",
    name: "Creating a new unique constraint",
    condition: "Found a new unique constraint and a new index",
    workaround: "`CREATE UNIQUE INDEX CONCURRENTLY`, then add the constraint using the index",
    effect: "This blocks all writes to the table while the index is being created and validated",
};
pub const NEW_EXCLUSION_CONSTRAINT_FOUND: StaticHintData = StaticHintData {
    id: "E8",
    name: "Creating a new exclusion constraint",
    condition: "Found a new exclusion constraint",
    workaround: "There is no safe way to add an exclusion constraint to an existing table",
    effect:
        "This blocks all reads and writes to the table while the constraint index is being created",
};
pub const TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT: StaticHintData = StaticHintData {
    id: "E9",
    name: "Taking dangerous lock without timeout",
    condition: "A lock that would block many common operations was taken without a timeout",
    workaround: "Run `SET LOCAL lock_timeout = '2s';` before the statement and retry the migration if necessary",
    effect: "This can block all other operations on the table indefinitely if any other transaction \
    holds a conflicting lock while `idle in transaction` or `active`",

};
pub const REWROTE_TABLE_WHILE_HOLDING_DANGEROUS_LOCK: StaticHintData = StaticHintData {
    id: "E10",
    name: "Rewrote table or index while holding dangerous lock",
    condition: "A table or index was rewritten while holding a lock that blocks many operations",
    workaround: "Build a new table or index, write to both, then swap them",
    effect: "This blocks many operations on the table or index while the rewrite is in progress",
};
pub const ADDED_SERIAL_OR_STORED_GENERATED_COLUMN: StaticHintData = StaticHintData {
    id: "E11",
    name: "Adding a `SERIAL` or `GENERATED ... STORED` column",
    condition: "A new column was added with a `SERIAL` or `GENERATED` type",
    workaround: "Can not be done without a table rewrite",
    effect: "This blocks all table access until the table is rewritten",
};
