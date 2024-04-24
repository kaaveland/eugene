use crate::pg_types::lock_modes::LockMode::*;

/// A lock mode in PostgreSQL, see [the documentation](https://www.postgresql.org/docs/current/explicit-locking.html)
#[derive(Eq, PartialEq, Debug, Copy, Clone, Hash)]
pub enum LockMode {
    AccessShare,
    RowShare,
    RowExclusive,
    ShareUpdateExclusive,
    Share,
    ShareRowExclusive,
    Exclusive,
    AccessExclusive,
}

/// All lock modes in PostgreSQL
pub const LOCK_MODES: [LockMode; 8] = [
    AccessShare,
    RowShare,
    RowExclusive,
    ShareUpdateExclusive,
    Share,
    ShareRowExclusive,
    Exclusive,
    AccessExclusive,
];

/// These are the operations that a lock type enable towards a table/index. This information isn't
/// perfect, for example ALTER TABLE is repeated several times because there are so many variants of
/// the statement and some of them can work with lesser locks, f. ex:
/// ALTER TABLE ... SET STATISTICS takes SHARE UPDATE EXCLUSIVE
/// ALTER TABLE ... ADD FOREIGN KEY takes SHARE ROW EXCLUSIVE
/// ALTER TABLE ... VALIDATE CONSTRAINT takes SHARE UPDATE EXCLUSIVE
/// But ACCESS EXCLUSIVE is necessary for most forms of ALTER TABLE.
/// See [the documentation](https://www.postgresql.org/docs/current/explicit-locking.html)
mod capabilities {
    pub const ACCESS_SHARE: [&str; 1] = ["SELECT"];
    pub const ROW_SHARE: [&str; 4] = [
        "FOR UPDATE",
        "FOR NO KEY UPDATE",
        "FOR SHARE",
        "FOR KEY SHARE",
    ];
    pub const ROW_EXCLUSIVE: [&str; 4] = ["UPDATE", "DELETE", "INSERT", "MERGE"];
    pub const SHARE_UPDATE_EXCLUSIVE: [&str; 7] = [
        "VACUUM",
        "ANALYZE",
        "CREATE INDEX CONCURRENTLY",
        "CREATE STATISTICS",
        "REINDEX CONCURRENTLY",
        "ALTER INDEX",
        "ALTER TABLE",
    ];
    pub const SHARE: [&str; 1] = ["CREATE INDEX"];
    pub const SHARE_ROW_EXCLUSIVE: [&str; 2] = ["CREATE TRIGGER", "ALTER TABLE"];
    pub const EXCLUSIVE: [&str; 1] = ["REFRESH MATERIALIZED VIEW CONCURRENTLY"];
    pub const ACCESS_EXCLUSIVE: [&str; 7] = [
        "ALTER TABLE",
        "DROP TABLE",
        "TRUNCATE",
        "REINDEX",
        "CLUSTER",
        "VACUUM FULL",
        "REFRESH MATERIALIZED VIEW",
    ];
}

impl std::fmt::Display for LockMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_db_str())
    }
}

/// These capabilities are often required by oltp applications and it could be dangerous to block them.
pub const QUERY_CAPABILITIES: [&str; 9] = [
    "SELECT",
    "FOR UPDATE",
    "FOR NO KEY UPDATE",
    "FOR SHARE",
    "FOR KEY SHARE",
    "UPDATE",
    "DELETE",
    "INSERT",
    "MERGE",
];

impl LockMode {
    /// Convert from a string that may be found in the `pg_locks.mode` column
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "AccessShareLock" => Some(AccessShare),
            "RowShareLock" => Some(RowShare),
            "RowExclusiveLock" => Some(RowExclusive),
            "ShareUpdateExclusiveLock" => Some(ShareUpdateExclusive),
            "ShareLock" => Some(Share),
            "ShareRowExclusiveLock" => Some(ShareRowExclusive),
            "ExclusiveLock" => Some(Exclusive),
            "AccessExclusiveLock" => Some(AccessExclusive),
            _ => None,
        }
    }
    /// Convert to str that may be found in the `pg_locks.mode` column
    pub fn to_db_str(&self) -> &'static str {
        match self {
            AccessShare => "AccessShareLock",
            RowShare => "RowShareLock",
            RowExclusive => "RowExclusiveLock",
            ShareUpdateExclusive => "ShareUpdateExclusiveLock",
            Share => "ShareLock",
            ShareRowExclusive => "ShareRowExclusiveLock",
            Exclusive => "ExclusiveLock",
            AccessExclusive => "AccessExclusiveLock",
        }
    }
    /// What lock modes this lock mode conflicts with.
    pub fn conflicts_with(&self) -> &[LockMode] {
        match self {
            AccessShare => &[AccessExclusive],
            RowShare => &[Exclusive, AccessExclusive],
            RowExclusive => &[Share, ShareRowExclusive, Exclusive, AccessExclusive],
            ShareUpdateExclusive => &[
                ShareUpdateExclusive,
                Share,
                ShareRowExclusive,
                Exclusive,
                AccessExclusive,
            ],
            Share => &[
                RowExclusive,
                ShareUpdateExclusive,
                ShareRowExclusive,
                Exclusive,
                AccessExclusive,
            ],
            ShareRowExclusive => &[
                RowExclusive,
                ShareUpdateExclusive,
                Share,
                ShareRowExclusive,
                Exclusive,
                AccessExclusive,
            ],
            Exclusive => &[
                RowShare,
                RowExclusive,
                ShareUpdateExclusive,
                Share,
                ShareRowExclusive,
                Exclusive,
                AccessExclusive,
            ],
            AccessExclusive => &LOCK_MODES,
        }
    }
    /// What capabilities this lock mode is used for.
    pub fn capabilities(&self) -> &[&str] {
        match self {
            AccessShare => &capabilities::ACCESS_SHARE,
            RowShare => &capabilities::ROW_SHARE,
            RowExclusive => &capabilities::ROW_EXCLUSIVE,
            ShareUpdateExclusive => &capabilities::SHARE_UPDATE_EXCLUSIVE,
            Share => &capabilities::SHARE,
            ShareRowExclusive => &capabilities::SHARE_ROW_EXCLUSIVE,
            Exclusive => &capabilities::EXCLUSIVE,
            AccessExclusive => &capabilities::ACCESS_EXCLUSIVE,
        }
    }
    /// What queries this lock mode blocks.
    pub fn blocked_queries(&self) -> Vec<&str> {
        self.conflicts_with()
            .iter()
            .flat_map(|lock| lock.capabilities().iter().copied())
            .filter(|cap| QUERY_CAPABILITIES.contains(cap))
            .collect()
    }
    /// What DDL statements this lock mode blocks.
    pub fn blocked_ddl(&self) -> Vec<&str> {
        self.conflicts_with()
            .iter()
            .flat_map(|lock| lock.capabilities().iter().copied())
            .filter(|cap| !QUERY_CAPABILITIES.contains(cap))
            .collect()
    }

    pub fn dangerous(&self) -> bool {
        self.conflicts_with()
            .iter()
            .flat_map(|lock| lock.capabilities().iter().copied())
            .filter(|cap| QUERY_CAPABILITIES.contains(cap))
            .count()
            > 0
    }
}

#[cfg(test)]
mod tests {
    use crate::pg_types::lock_modes::LOCK_MODES;

    #[test]
    fn test_locks_that_block_select_are_dangerous() {
        LOCK_MODES
            .iter()
            .filter(|lock| lock.capabilities().contains(&"SELECT"))
            .flat_map(|lock| lock.conflicts_with().iter())
            .for_each(|lock| assert!(lock.dangerous()));
    }

    #[test]
    fn test_locks_that_block_update_are_dangerous() {
        LOCK_MODES
            .iter()
            .filter(|lock| lock.capabilities().contains(&"UPDATE"))
            .flat_map(|lock| lock.conflicts_with().iter())
            .for_each(|lock| assert!(lock.dangerous()));
    }

    #[test]
    fn test_locks_that_block_for_update_are_dangerous() {
        LOCK_MODES
            .iter()
            .filter(|lock| lock.capabilities().contains(&"FOR UPDATE"))
            .flat_map(|lock| lock.conflicts_with().iter())
            .for_each(|lock| assert!(lock.dangerous()));
    }
}
