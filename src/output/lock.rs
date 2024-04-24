use crate::pg_types::locks::Lock;
use serde::Serialize;
use std::fmt;
use std::fmt::Display;

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct TerseLock<'a> {
    mode: &'a str,
    schema: &'a str,
    object_name: &'a str,
}

impl Display for TerseLock<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} on {}.{}", self.mode, self.schema, self.object_name)
    }
}

impl<'a> From<&'a Lock> for TerseLock<'a> {
    fn from(value: &'a Lock) -> Self {
        TerseLock {
            mode: value.mode.to_db_str(),
            schema: value.target().schema.as_str(),
            object_name: value.target().object_name.as_str(),
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct NormalLock<'a> {
    #[serde(flatten)]
    terse: TerseLock<'a>,
    blocked_queries: Vec<&'a str>,
}

impl Display for NormalLock<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.terse)?;
        if !self.blocked_queries.is_empty() {
            write!(f, " would block:\n  {:?}", self.blocked_queries)?;
        }
        Ok(())
    }
}

impl<'a> From<&'a Lock> for NormalLock<'a> {
    fn from(value: &'a Lock) -> Self {
        NormalLock {
            terse: value.into(),
            blocked_queries: value.blocked_queries(),
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct DetailedLock<'a> {
    #[serde(flatten)]
    normal: NormalLock<'a>,
    rel_kind: &'a str,
    blocked_ddl: Vec<&'a str>,
}

impl Display for DetailedLock<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} on {} {}.{}",
            self.normal.terse.mode,
            self.rel_kind,
            self.normal.terse.schema,
            self.normal.terse.object_name
        )?;
        writeln!(f, " would block:\n  {:?}", self.normal.blocked_queries)?;
        write!(
            f,
            "{} on {} {}.{}",
            self.normal.terse.mode,
            self.rel_kind,
            self.normal.terse.schema,
            self.normal.terse.object_name
        )?;
        write!(f, " would block DDL:\n  {:?}", self.blocked_ddl)?;

        Ok(())
    }
}

impl<'a> From<&'a Lock> for DetailedLock<'a> {
    fn from(value: &'a Lock) -> Self {
        DetailedLock {
            normal: value.into(),
            rel_kind: value.target().rel_kind.as_str(),
            blocked_ddl: value.mode.blocked_ddl(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_play_with_display_output_format() {
        let lock = Lock::new("public", "table", "ExclusiveLock", 'r').unwrap();
        let terse = TerseLock::from(&lock);
        assert_eq!(format!("{}", terse), "ExclusiveLock on public.table");
        let normal = NormalLock::from(&lock);
        assert_eq!(format!("{}", normal), "ExclusiveLock on public.table would block:
  [\"FOR UPDATE\", \"FOR NO KEY UPDATE\", \"FOR SHARE\", \"FOR KEY SHARE\", \"UPDATE\", \"DELETE\", \"INSERT\", \"MERGE\"]");
        let detailed = DetailedLock::from(&lock);
        assert_eq!(format!("{}", detailed), "ExclusiveLock on Table public.table would block:
  [\"FOR UPDATE\", \"FOR NO KEY UPDATE\", \"FOR SHARE\", \"FOR KEY SHARE\", \"UPDATE\", \"DELETE\", \"INSERT\", \"MERGE\"]
ExclusiveLock on Table public.table would block DDL:
  [\"VACUUM\", \"ANALYZE\", \"CREATE INDEX CONCURRENTLY\", \"CREATE STATISTICS\", \"REINDEX CONCURRENTLY\", \"ALTER INDEX\", \"ALTER TABLE\", \"CREATE INDEX\", \"CREATE TRIGGER\", \"ALTER TABLE\", \"REFRESH MATERIALIZED VIEW CONCURRENTLY\", \"ALTER TABLE\", \"DROP TABLE\", \"TRUNCATE\", \"REINDEX\", \"CLUSTER\", \"VACUUM FULL\", \"REFRESH MATERIALIZED VIEW\"]");
    }
}
