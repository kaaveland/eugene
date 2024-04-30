use serde::Serialize;
use std::fmt::{Display, Formatter};

use crate::output::lock::{DetailedLock, NormalLock, TerseLock};
use crate::pg_types::locks::Lock;
use crate::tracing::SqlStatementTrace;

pub struct SqlStatementCtx<'a> {
    pub(crate) trace: &'a SqlStatementTrace,
    pub(crate) statement_number: usize,
    pub(crate) locks_before: Vec<&'a Lock>,
    pub(crate) extra: bool,
}

impl<'a> SqlStatementCtx<'a> {
    pub fn sql(&self) -> &'a str {
        self.trace.sql.as_str()
    }

    pub fn locks_taken<T: From<&'a Lock>>(&self) -> Vec<T> {
        self.trace
            .locks_taken
            .iter()
            .filter(|lock| self.extra || lock.mode.dangerous())
            .map(|lock| lock.into())
            .collect()
    }
    pub fn locks_before<T: From<&'a Lock>>(&self) -> Vec<T> {
        self.locks_before
            .iter()
            .map(|lock| (*lock).into())
            .collect()
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct StatementBase {
    statement_number: usize,
    pub(crate) duration_millis: u64,
}
impl<'a> From<&'a SqlStatementCtx<'a>> for StatementBase {
    fn from(value: &'a SqlStatementCtx) -> Self {
        StatementBase {
            statement_number: value.statement_number,
            duration_millis: value.trace.duration.as_millis() as u64,
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct TerseSqlStatement<'a> {
    #[serde(flatten)]
    base: StatementBase,
    locks_taken: Vec<TerseLock<'a>>,
}

impl Display for TerseSqlStatement<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let locks: Vec<String> = self
            .locks_taken
            .iter()
            .map(|lock| format!("{}", lock))
            .collect();
        write!(
            f,
            "Statement #{} took {}ms and",
            self.base.statement_number, self.base.duration_millis
        )?;
        if locks.is_empty() {
            write!(f, " took no locks")
        } else {
            write!(f, " took locks: {}", locks.join(", "))
        }
    }
}

impl<'a> From<&'a SqlStatementCtx<'a>> for TerseSqlStatement<'a> {
    fn from(value: &'a SqlStatementCtx<'a>) -> Self {
        TerseSqlStatement {
            base: value.into(),
            locks_taken: value.locks_taken(),
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct NormalSqlStatement<'a> {
    #[serde(flatten)]
    pub(crate) base: StatementBase,
    sql: &'a str,
    locks_taken: Vec<NormalLock<'a>>,
    locks_held: Vec<NormalLock<'a>>,
}

impl Display for NormalSqlStatement<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Statement #{} took {}ms and",
            self.base.statement_number, self.base.duration_millis
        )?;
        let locks = self
            .locks_taken
            .iter()
            .map(|lock| format!("{}", lock))
            .collect::<Vec<String>>();
        if locks.is_empty() {
            writeln!(f, " took no locks")
        } else {
            writeln!(f, " took locks:\n  {}", locks.join("\n  "))
        }?;
        write!(f, "Statement was:\n{}", self.sql)
    }
}

impl<'a> From<&'a SqlStatementCtx<'a>> for NormalSqlStatement<'a> {
    fn from(value: &'a SqlStatementCtx<'a>) -> Self {
        NormalSqlStatement {
            base: value.into(),
            sql: value.trace.sql.as_str(),
            locks_taken: value.locks_taken(),
            locks_held: value.locks_before(),
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct VerboseSqlStatement<'a> {
    #[serde(flatten)]
    pub(crate) base: StatementBase,
    sql: &'a str,
    locks_taken: Vec<DetailedLock<'a>>,
    locks_held: Vec<DetailedLock<'a>>,
}

impl Display for VerboseSqlStatement<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Statement #{} took {}ms and",
            self.base.statement_number, self.base.duration_millis
        )?;
        let locks = self
            .locks_taken
            .iter()
            .map(|lock| format!("{}", lock))
            .collect::<Vec<String>>();
        if locks.is_empty() {
            write!(f, " took no locks")
        } else {
            write!(f, " took locks:\n    {}", locks.join("\n    "))
        }
    }
}

impl<'a> From<&'a SqlStatementCtx<'a>> for VerboseSqlStatement<'a> {
    fn from(value: &'a SqlStatementCtx<'a>) -> Self {
        VerboseSqlStatement {
            base: value.into(),
            sql: value.trace.sql.as_str(),
            locks_taken: value.locks_taken(),
            locks_held: value.locks_before(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pg_types::lock_modes::LockMode;
    use crate::pg_types::lock_modes::LockMode::AccessShare;
    use crate::pg_types::locks::LockableTarget;
    use crate::pg_types::relkinds::RelKind;
    #[test]
    fn check_sql_statement_display_output() {
        let before = Lock {
            mode: LockMode::Exclusive,
            target: LockableTarget {
                schema: "public".to_string(),
                object_name: "foo".to_string(),
                rel_kind: RelKind::Table,
            },
        };
        let ctx = SqlStatementCtx {
            trace: &SqlStatementTrace {
                sql: "SELECT * FROM foo".to_string(),
                duration: std::time::Duration::from_millis(100),
                locks_taken: vec![Lock {
                    mode: AccessShare,
                    target: LockableTarget {
                        schema: "public".to_string(),
                        object_name: "foo".to_string(),
                        rel_kind: RelKind::Table,
                    },
                }],
                start_time: std::time::Instant::now(),
            },
            statement_number: 1,
            locks_before: vec![&before],
            extra: true,
        };
        let terse = TerseSqlStatement::from(&ctx);
        assert_eq!(
            format!("{}", terse),
            "Statement #1 took 100ms and took locks: AccessShareLock on public.foo"
        );
        let normal = NormalSqlStatement::from(&ctx);
        assert_eq!(
            format!("{}", normal),
            "Statement #1 took 100ms and took locks:
  AccessShareLock on public.foo
Statement was:
SELECT * FROM foo"
        );
    }
}
