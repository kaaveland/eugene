pub fn hint_url<S: AsRef<str>>(id: S) -> String {
    format!("https://kaveland.no/eugene/hints/{}/", id.as_ref())
}

pub trait HintId {
    fn id(&self) -> &str;
    fn url(&self) -> String {
        hint_url::<&str>(self.id())
    }
}

pub struct StaticHintData {
    pub id: &'static str,
    pub name: &'static str,
    pub condition: &'static str,
    pub effect: &'static str,
    pub workaround: &'static str,
    pub bad_example: &'static str,
    pub good_example: Option<&'static str>,
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
    bad_example: include_str!("../examples/E1/bad.sql"),
    good_example: Some(include_str!("../examples/E1/good.sql")),
};
pub const MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK: StaticHintData = StaticHintData {
    id: "E2",
    name: "Validating table with a new `NOT NULL` column",
    condition: "A column was changed from `NULL` to `NOT NULL`",
    workaround: "Add a `CHECK` constraint as `NOT VALID`, validate it later, then make the column `NOT NULL`",
    effect: "This blocks all table access until all rows are validated",
    bad_example: include_str!("../examples/E2/bad.sql"),
    good_example: Some(include_str!("../examples/E2/good.sql")),
};
pub const ADD_JSON_COLUMN: StaticHintData = StaticHintData {
    id: "E3",
    name: "Add a new JSON column",
    condition: "A new column of type `json` was added to a table",
    workaround: "Use the `jsonb` type instead, it supports all use-cases of `json` and is more robust and compact",
    effect: "This breaks `SELECT DISTINCT` queries or other operations that need equality checks on the column",
    bad_example: include_str!("../examples/E3/bad.sql"),
    good_example: Some(include_str!("../examples/E3/good.sql")),
};
pub const RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE: StaticHintData = StaticHintData {
    id: "E4",
    name: "Running more statements after taking `AccessExclusiveLock`",
    condition: "A transaction that holds an `AccessExclusiveLock` started a new statement",
    workaround: "Run this statement in a new transaction",
    effect: "This blocks all access to the table for the duration of this statement",
    bad_example: include_str!("../examples/E4/bad.sql"),
    good_example: Some(include_str!("../examples/E4/good.sql")),
};
pub const TYPE_CHANGE_REQUIRES_TABLE_REWRITE: StaticHintData = StaticHintData {
    id: "E5",
    name: "Type change requiring table rewrite",
    condition: "A column was changed to a data type that isn't binary compatible",
    workaround: "Add a new column, update it in batches, and drop the old column",
    effect: "This causes a full table rewrite while holding a lock that prevents all other use of the table",
    bad_example: include_str!("../examples/E5/bad.sql"),
    good_example: Some(include_str!("../examples/E5/good.sql")),
};
pub const NEW_INDEX_ON_EXISTING_TABLE_IS_NONCONCURRENT: StaticHintData = StaticHintData {
    id: "E6",
    name: "Creating a new index on an existing table",
    condition: "A new index was created on an existing table without the `CONCURRENTLY` keyword",
    workaround: "Run `CREATE INDEX CONCURRENTLY` instead of `CREATE INDEX`",
    effect: "This blocks all writes to the table while the index is being created",
    bad_example: include_str!("../examples/E6/bad.sql"),
    good_example: Some(include_str!("../examples/E6/good.sql")),
};
pub const NEW_UNIQUE_CONSTRAINT_CREATED_INDEX: StaticHintData = StaticHintData {
    id: "E7",
    name: "Creating a new unique constraint",
    condition: "Adding a new unique constraint implicitly creates index",
    workaround: "`CREATE UNIQUE INDEX CONCURRENTLY`, then add the constraint using the index",
    effect: "This blocks all writes to the table while the index is being created and validated",
    bad_example: include_str!("../examples/E7/bad.sql"),
    good_example: Some(include_str!("../examples/E7/good.sql")),
};
pub const NEW_EXCLUSION_CONSTRAINT_FOUND: StaticHintData = StaticHintData {
    id: "E8",
    name: "Creating a new exclusion constraint",
    condition: "Found a new exclusion constraint",
    workaround: "There is no safe way to add an exclusion constraint to an existing table",
    effect:
        "This blocks all reads and writes to the table while the constraint index is being created",
    bad_example: include_str!("../examples/E8/bad.sql"),
    good_example: None,
};
pub const TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT: StaticHintData = StaticHintData {
    id: "E9",
    name: "Taking dangerous lock without timeout",
    condition: "A lock that would block many common operations was taken without a timeout",
    workaround: "Run `SET LOCAL lock_timeout = '2s';` before the statement and retry the migration if necessary",
    effect: "This can block all other operations on the table indefinitely if any other transaction \
    holds a conflicting lock while `idle in transaction` or `active`",
    bad_example: include_str!("../examples/E9/bad.sql"),
    good_example: Some(include_str!("../examples/E9/good.sql")),
};
pub const REWROTE_TABLE_WHILE_HOLDING_DANGEROUS_LOCK: StaticHintData = StaticHintData {
    id: "E10",
    name: "Rewrote table or index while holding dangerous lock",
    condition: "A table or index was rewritten while holding a lock that blocks many operations",
    workaround: "Build a new table or index, write to both, then swap them",
    effect: "This blocks many operations on the table or index while the rewrite is in progress",
    bad_example: include_str!("../examples/E10/bad.sql"),
    good_example: Some(include_str!("../examples/E10/good.sql")),
};
pub const ADDED_SERIAL_OR_STORED_GENERATED_COLUMN: StaticHintData = StaticHintData {
    id: "E11",
    name: "Adding a `SERIAL` or `GENERATED ... STORED` column",
    condition: "A new column was added with a `SERIAL` or `GENERATED` type",
    workaround: "Can not be done without a table rewrite",
    effect: "This blocks all table access until the table is rewritten",
    bad_example: include_str!("../examples/E11/bad.sql"),
    good_example: None,
};
pub const MULTIPLE_ALTER_TABLES_WHERE_ONE_WILL_DO: StaticHintData = StaticHintData {
    id: "W12",
    name: "Multiple `ALTER TABLE` statements where one will do",
    condition: "Multiple `ALTER TABLE` statements targets the same table",
    workaround: "Combine the statements into one, separating the action with commas",
    effect: "If the statements require table scans, there will be more scans than necessary",
    bad_example: include_str!("../examples/W12/bad.sql"),
    good_example: Some(include_str!("../examples/W12/good.sql")),
};
pub const CREATING_ENUM: StaticHintData = StaticHintData {
    id: "W13",
    name: "Creating an enum",
    condition: "A new enum was created",
    workaround: "Use a foreign key to a lookup table instead",
    effect: "Removing values from an enum requires difficult migrations, and associating more data with an enum value is difficult",
    bad_example: include_str!("../examples/W13/bad.sql"),
    good_example: Some(include_str!("../examples/W13/good.sql")),
};
pub const ADD_PRIMARY_KEY_USING_INDEX: StaticHintData = StaticHintData {
    id: "W14",
    name: "Adding a primary key using an index",
    condition: "A primary key was added using an index on the table",
    workaround: "Make sure that all the columns in the index are already `NOT NULL`",
    effect: "This can cause postgres to alter the index columns to be `NOT NULL`",
    bad_example: include_str!("../examples/W14/bad.sql"),
    good_example: Some(include_str!("../examples/W14/good.sql")),
};
pub const FOREIGN_KEY_NOT_BACKED_BY_INDEX: StaticHintData = StaticHintData {
    id: "E15",
    name: "Missing index",
    condition: "A foreign key is missing a complete index on the referencing side",
    effect: "Updates and deletes on the referenced table may cause table scan on referencing table",
    workaround: "Create the missing index",
    bad_example: include_str!("../examples/E15/bad.sql"),
    good_example: Some(include_str!("../examples/E15/good.sql")),
};

pub const ALL: &[&StaticHintData] = &[
    &VALIDATE_CONSTRAINT_WITH_LOCK,
    &MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK,
    &ADD_JSON_COLUMN,
    &RUNNING_STATEMENT_WHILE_HOLDING_ACCESS_EXCLUSIVE,
    &TYPE_CHANGE_REQUIRES_TABLE_REWRITE,
    &NEW_INDEX_ON_EXISTING_TABLE_IS_NONCONCURRENT,
    &NEW_UNIQUE_CONSTRAINT_CREATED_INDEX,
    &NEW_EXCLUSION_CONSTRAINT_FOUND,
    &TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT,
    &REWROTE_TABLE_WHILE_HOLDING_DANGEROUS_LOCK,
    &ADDED_SERIAL_OR_STORED_GENERATED_COLUMN,
    &MULTIPLE_ALTER_TABLES_WHERE_ONE_WILL_DO,
    &CREATING_ENUM,
    &ADD_PRIMARY_KEY_USING_INDEX,
    &FOREIGN_KEY_NOT_BACKED_BY_INDEX,
];

pub fn data_by_id<S: AsRef<str>>(id: S) -> Option<&'static StaticHintData> {
    ALL.iter().find(|hint| hint.id == id.as_ref()).copied()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_no_duplicated_id_or_name() {
        let mut ids = std::collections::HashSet::new();
        let mut names = std::collections::HashSet::new();
        for hint in super::ALL {
            assert!(ids.insert(hint.id), "Duplicated id: {}", hint.id);
            assert!(ids.insert(&hint.id[1..]), "Duplicated id: {}", hint.id);
            assert!(names.insert(hint.name), "Duplicated name: {}", hint.name);
        }
    }
}
