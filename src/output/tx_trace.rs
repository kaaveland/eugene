use crate::output::sql_statement::{
    NormalSqlStatement, SqlStatementCtx, TerseSqlStatement, VerboseSqlStatement,
};
use crate::tracing::TxLockTracer;
use serde::Serialize;

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
impl<'a> From<&'a TxTraceData<'a>> for DetailedTxTrace<'a> {
    fn from(value: &'a TxTraceData) -> Self {
        DetailedTxTrace {
            name: value.name,
            sql_statements: value.sql_statements.iter().map(|ctx| ctx.into()).collect(),
        }
    }
}
