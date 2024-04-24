use crate::output::sql_statement::{
    NormalSqlStatement, SqlStatementCtx, TerseSqlStatement, VerboseSqlStatement,
};
use crate::tracing::TxLockTracer;
use serde::Serialize;
use std::fmt::Display;

pub struct TxTraceData<'a> {
    pub(crate) name: Option<&'a str>,
    pub(crate) sql_statements: Vec<SqlStatementCtx<'a>>,
}

impl<'a> TxTraceData<'a> {
    pub fn new(trace: &'a TxLockTracer, extra: bool) -> Self {
        let mut sql_statements = vec![];
        let mut locks = vec![];
        for statement in &trace.statements {
            let ctx = SqlStatementCtx {
                statement_number: sql_statements.len() + 1,
                trace: statement,
                locks_before: locks.clone(),
                extra,
            };
            locks.extend(
                statement
                    .locks_taken
                    .iter()
                    .filter(|lock| extra || lock.mode.dangerous()),
            );
            sql_statements.push(ctx);
        }

        TxTraceData {
            name: trace.name.as_deref(),
            sql_statements,
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct TerseTxTrace<'a> {
    name: Option<&'a str>,
    sql_statements: Vec<TerseSqlStatement<'a>>,
}

fn display_statements<T: Display>(
    f: &mut std::fmt::Formatter,
    statements: &[T],
) -> std::fmt::Result {
    if !statements.is_empty() {
        writeln!(f)?;
        let statements = statements
            .iter()
            .map(|statement| format!("{}", statement))
            .collect::<Vec<String>>();
        write!(f, "{}", statements.join("\n"))?;
    }
    Ok(())
}
impl Display for TerseTxTrace<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Traced: {}", self.name.unwrap_or("unnamed"))?;
        display_statements(f, &self.sql_statements)
    }
}

impl<'a> From<&'a TxTraceData<'a>> for TerseTxTrace<'a> {
    fn from(value: &'a TxTraceData) -> Self {
        TerseTxTrace {
            name: value.name,
            sql_statements: value.sql_statements.iter().map(|ctx| ctx.into()).collect(),
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct NormalTxTrace<'a> {
    pub(crate) name: Option<&'a str>,
    sql_statements: Vec<NormalSqlStatement<'a>>,
}
impl Display for NormalTxTrace<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let total_time = self
            .sql_statements
            .iter()
            .map(|statement| statement.base.duration_millis)
            .sum::<u64>();
        write!(
            f,
            "Traced: {} for {}ms",
            self.name.unwrap_or("unnamed"),
            total_time
        )?;
        display_statements(f, &self.sql_statements)
    }
}
impl<'a> From<&'a TxTraceData<'a>> for NormalTxTrace<'a> {
    fn from(value: &'a TxTraceData) -> Self {
        NormalTxTrace {
            name: value.name,
            sql_statements: value.sql_statements.iter().map(|ctx| ctx.into()).collect(),
        }
    }
}
#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct DetailedTxTrace<'a> {
    name: Option<&'a str>,
    sql_statements: Vec<VerboseSqlStatement<'a>>,
}
impl Display for DetailedTxTrace<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let total_time = self
            .sql_statements
            .iter()
            .map(|statement| statement.base.duration_millis)
            .sum::<u64>();
        write!(
            f,
            "Traced: {} for {}ms",
            self.name.unwrap_or("unnamed"),
            total_time
        )?;
        display_statements(f, &self.sql_statements)
    }
}
impl<'a> From<&'a TxTraceData<'a>> for DetailedTxTrace<'a> {
    fn from(value: &'a TxTraceData) -> Self {
        DetailedTxTrace {
            name: value.name,
            sql_statements: value.sql_statements.iter().map(|ctx| ctx.into()).collect(),
        }
    }
}
