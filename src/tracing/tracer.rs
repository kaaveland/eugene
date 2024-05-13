use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use itertools::Itertools;
use postgres::types::Oid;
use postgres::Transaction;

use crate::hints;
use crate::output::output_format::Hint;
use crate::pg_types::locks::{Lock, LockableTarget};
use crate::tracing::queries;
use crate::tracing::queries::{ColumnIdentifier, ColumnMetadata, Constraint, RelfileId};

/// A trace of a single SQL statement, including the locks taken and the duration of the statement.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct SqlStatementTrace {
    /// The SQL statement that was executed.
    pub(crate) sql: String,
    /// New locks taken by this statement.
    pub(crate) locks_taken: Vec<Lock>,
    /// The time the statement started executing.
    pub(crate) start_time: Instant,
    /// The duration of the statement.
    pub(crate) duration: Duration,
    /// Columns that were added
    pub(crate) added_columns: Vec<(ColumnIdentifier, ColumnMetadata)>,
    /// Columns that were modified
    pub(crate) modified_columns: Vec<(ColumnIdentifier, ModifiedColumn)>,
    /// Constraints that were added
    pub(crate) added_constraints: Vec<Constraint>,
    /// Constraints that were modified
    pub(crate) modified_constraints: Vec<(Oid, ModifiedConstraint)>,
    /// Database objects that were created by this statement
    pub(crate) created_objects: Vec<LockableTarget>,
    /// The `lock_timeout` that was active in postgres when `sql` started to execute
    pub(crate) lock_timeout_millis: u64,

    /// Rewritten database objects
    pub(crate) rewritten_objects: Vec<RelfileId>,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ModifiedColumn {
    pub(crate) old: ColumnMetadata,
    pub(crate) new: ColumnMetadata,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ModifiedConstraint {
    pub(crate) old: Constraint,
    pub(crate) new: Constraint,
}

/// A trace of a transaction, including all SQL statements executed and the locks taken by each one.
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct TxLockTracer {
    /// The name of the transaction, if any, typically the file name.
    pub(crate) name: Option<String>,
    /// The initial set of objects that are interesting to track locks for.
    pub(crate) initial_objects: HashSet<Oid>,
    /// The list of all SQL statements executed so far in the transaction.
    pub(crate) statements: Vec<SqlStatementTrace>,

    /// All hints triggered by statements in this transaction, grouped by statement.
    pub(crate) triggered_hints: Vec<Vec<Hint>>,
    /// All locks taken so far in the transaction.
    pub(crate) all_locks: HashSet<Lock>,
    /// The time the trace started
    pub(crate) trace_start: DateTime<Local>,
    /// All columns in the database, along with their metadata
    pub(crate) columns: HashMap<ColumnIdentifier, ColumnMetadata>,
    /// All constraints in the database
    pub(crate) constraints: HashMap<Oid, Constraint>,
    /// Is the trace from one or more `CONCURRENTLY` statements that must run outside transactions?
    pub(crate) concurrent: bool,

    /// Database objects that have been created in the transaction
    pub(crate) created_objects: HashSet<Oid>,

    /// The relation file IDs of all relations in the database
    pub(crate) relfile_ids: HashMap<Oid, u32>,
}

pub struct StatementCtx<'a> {
    pub(crate) sql_statement_trace: &'a SqlStatementTrace,
    pub(crate) transaction: &'a TxLockTracer,
}

impl<'a> StatementCtx<'a> {
    pub fn new_constraints(&self) -> impl Iterator<Item = &Constraint> {
        self.sql_statement_trace.added_constraints.iter()
    }
    pub fn altered_columns(&self) -> impl Iterator<Item = &(ColumnIdentifier, ModifiedColumn)> {
        self.sql_statement_trace.modified_columns.iter()
    }
    pub fn new_columns(&self) -> impl Iterator<Item = &ColumnMetadata> {
        self.sql_statement_trace
            .added_columns
            .iter()
            .map(|(_, col)| col)
    }
    pub fn locks_at_start(&self) -> impl Iterator<Item = &Lock> {
        self.transaction.all_locks.iter()
    }
    pub fn new_locks_taken(&self) -> impl Iterator<Item = &Lock> {
        self.sql_statement_trace.locks_taken.iter()
    }
    pub fn new_objects(&self) -> impl Iterator<Item = &LockableTarget> {
        self.sql_statement_trace.created_objects.iter()
    }
    pub fn lock_timeout_millis(&self) -> u64 {
        self.sql_statement_trace.lock_timeout_millis
    }
    pub fn constraints_on(&self, oid: Oid) -> impl Iterator<Item = &Constraint> {
        self.transaction
            .constraints
            .values()
            .filter(move |con| con.target == oid)
    }
    pub fn rewritten_objects(&self) -> impl Iterator<Item = &RelfileId> {
        self.sql_statement_trace.rewritten_objects.iter()
    }
}

impl TxLockTracer {
    /// Trace a single SQL statement, recording the locks taken and the duration of the statement.
    pub fn trace_sql_statement(&mut self, tx: &mut Transaction, sql: &str) -> Result<()> {
        // TODO: This is too big and should be refactored into more manageable pieces
        let start_time = Instant::now();
        let oid_vec = self.initial_objects.iter().copied().collect_vec();
        let lock_timeout = queries::get_lock_timeout(tx)?;
        tx.execute(sql, &[])
            .map_err(|err| anyhow!("{err} while executing {}", sql.to_owned()))?;
        let duration = start_time.elapsed();
        let locks_taken =
            queries::find_relevant_locks_in_current_transaction(tx, &self.initial_objects)?;
        let new_locks = queries::find_new_locks(&self.all_locks, &locks_taken);
        let relfile_ids = queries::fetch_all_rel_file_ids(tx, &oid_vec)?;

        let changed_ids: Vec<_> = relfile_ids
            .into_iter()
            .filter(|(oid, id)| self.relfile_ids.get(oid) != Some(&id.relfilenode))
            .map(|(_, id)| id)
            .collect();
        self.relfile_ids
            .extend(changed_ids.iter().map(|id| (id.oid, id.relfilenode)));

        let columns = queries::fetch_all_columns(tx, &oid_vec)?;
        let mut added_columns = Vec::new();
        let mut modified_columns = Vec::new();
        for (col_id, col) in columns.iter() {
            if let Some(pre_existing) = self.columns.get(col_id) {
                if pre_existing != col {
                    modified_columns.push((
                        *col_id,
                        ModifiedColumn {
                            new: col.clone(),
                            old: pre_existing.clone(),
                        },
                    ));
                }
            } else {
                added_columns.push((*col_id, col.clone()));
            }
        }
        self.columns = columns;

        let constraints = queries::fetch_constraints(tx, &oid_vec)?;
        let mut added_constraints = Vec::new();
        let mut modified_constraints = Vec::new();

        for (conid, con) in constraints.iter() {
            if let Some(pre_existing) = self.constraints.get(conid) {
                if pre_existing != con {
                    modified_constraints.push((
                        *conid,
                        ModifiedConstraint {
                            old: pre_existing.clone(),
                            new: con.clone(),
                        },
                    ));
                }
            } else {
                added_constraints.push(con.clone());
            }
        }
        self.constraints = constraints;
        let new_objects: Vec<_> = queries::fetch_lockable_objects(tx, &oid_vec)?
            .into_iter()
            .filter(|target| !self.created_objects.contains(&target.oid))
            .collect();
        self.created_objects
            .extend(new_objects.iter().map(|obj| obj.oid));

        let statement = SqlStatementTrace {
            sql: sql.to_string(),
            locks_taken: new_locks.into_iter().collect(),
            start_time,
            duration,
            added_columns,
            modified_columns,
            added_constraints,
            modified_constraints,
            created_objects: new_objects,
            lock_timeout_millis: lock_timeout,
            rewritten_objects: changed_ids,
        };
        let ctx = StatementCtx {
            sql_statement_trace: &statement,
            transaction: self,
        };
        let hints: Vec<_> = hints::run_hints(&ctx).collect();
        self.triggered_hints.push(hints);
        self.statements.push(statement);
        self.all_locks.extend(locks_taken.iter().cloned());
        Ok(())
    }
    /// Start a new lock tracing session.
    ///
    /// # Parameters
    /// * `name` - The name of the transaction, typically the file name.
    /// * `trace_targets` - The typically `Oid` of relations visible to other transactions.
    /// * `columns` - Initial columns in the database, to track changes.
    /// * `constraints` - Initial constraints in the database, to track changes.
    pub fn new(
        name: Option<String>,
        trace_targets: HashSet<Oid>,
        columns: HashMap<ColumnIdentifier, ColumnMetadata>,
        constraints: HashMap<Oid, Constraint>,
        relfile_ids: HashMap<Oid, u32>,
    ) -> Self {
        Self {
            name,
            initial_objects: trace_targets,
            statements: vec![],
            all_locks: HashSet::new(),
            trace_start: Local::now(),
            columns,
            constraints,
            concurrent: false,
            created_objects: Default::default(),
            triggered_hints: vec![],
            relfile_ids,
        }
    }

    /// Start a new lock tracing session for a `CONCURRENTLY` statement.
    ///
    /// # Parameters
    /// * `name` - The name of the transaction, typically the file name.
    /// * `statements` - The SQL statements to trace.
    ///
    /// This can not really do any tracing, as `CONCURRENTLY` statements must run outside transactions.
    pub fn tracer_for_concurrently<S: AsRef<str>>(
        name: Option<String>,
        statements: impl Iterator<Item = S>,
    ) -> Self {
        let mut out = Self {
            name,
            initial_objects: HashSet::new(),
            statements: statements
                .map(|s| SqlStatementTrace {
                    sql: s.as_ref().to_string(),
                    locks_taken: vec![],
                    start_time: Instant::now(),
                    duration: Duration::from_secs(0),
                    added_columns: vec![],
                    modified_columns: vec![],
                    added_constraints: vec![],
                    modified_constraints: vec![],
                    created_objects: vec![],
                    lock_timeout_millis: 0,
                    rewritten_objects: vec![],
                })
                .collect(),
            all_locks: HashSet::new(),
            trace_start: Local::now(),
            columns: HashMap::new(),
            constraints: HashMap::new(),
            concurrent: true,
            created_objects: Default::default(),
            triggered_hints: vec![],
            relfile_ids: Default::default(),
        };
        out.triggered_hints = vec![vec![]; out.statements.len()];
        out
    }
}
