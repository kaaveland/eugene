use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use itertools::Itertools;
use postgres::types::Oid;
use postgres::Transaction;

use crate::pg_types::contype::Contype;
use crate::pg_types::locks::{Lock, LockableTarget};

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
}

/// Enumerate all locks owned by the current transaction.
fn query_pg_locks_in_current_transaction(tx: &mut Transaction) -> Result<HashSet<Lock>> {
    let query = "SELECT n.nspname::text AS schema_name,
                c.relname::text AS object_name,
                c.relkind AS relkind,
                l.mode::text AS mode,
                c.oid AS oid
         FROM pg_locks l JOIN pg_class c ON c.oid = l.relation
           JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE l.locktype = 'relation' AND l.pid = pg_backend_pid();";
    let rows = tx.query(query, &[])?;
    let locks = rows
        .into_iter()
        .map(|row| {
            let schema: String = row.try_get(0)?;
            let object_name: String = row.try_get(1)?;
            let relkind: i8 = row.try_get(2)?;
            let mode: String = row.try_get(3)?;
            let oid: Oid = row.try_get(4)?;
            Lock::new(schema, object_name, mode, (relkind as u8) as char, oid)
                .map_err(|err| anyhow!("{err}"))
        })
        .collect::<Result<HashSet<Lock>, anyhow::Error>>()?;
    Ok(locks)
}

/// Find all locks in the current transaction that are relevant to the given set of objects.
fn find_relevant_locks_in_current_transaction(
    tx: &mut Transaction,
    relevant_objects: &HashSet<Oid>,
) -> Result<HashSet<Lock>> {
    let current_locks = query_pg_locks_in_current_transaction(tx)?;
    Ok(current_locks
        .into_iter()
        .filter(|lock| relevant_objects.contains(&lock.target_oid()))
        .collect())
}

/// Return the locks that are new in the new set of locks compared to the old set.
fn find_new_locks(old_locks: &HashSet<Lock>, new_locks: &HashSet<Lock>) -> HashSet<Lock> {
    let old = old_locks
        .iter()
        .map(|lock| (lock.target_oid(), lock.mode))
        .collect::<HashSet<_>>();
    new_locks
        .iter()
        .filter(|lock| !old.contains(&(lock.target_oid(), lock.mode)))
        .cloned()
        .collect()
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub struct ColumnIdentifier {
    oid: Oid,
    attnum: i32,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ColumnMetadata {
    name: String,
    nullable: bool,
    typename: String,
    max_len: Option<u32>,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ModifiedColumn {
    old: ColumnMetadata,
    new: ColumnMetadata,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Constraint {
    schema_name: String,
    table_name: String,
    constraint_type: Contype,
    name: String,
    expression: Option<String>,
    valid: bool,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ModifiedConstraint {
    old: Constraint,
    new: Constraint,
}

/// A trace of a transaction, including all SQL statements executed and the locks taken by each one.
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct TxLockTracer {
    /// The name of the transaction, if any, typically the file name.
    pub(crate) name: Option<String>,
    /// The initial set of objects that are interesting to track locks for.
    initial_objects: HashSet<Oid>,
    /// The list of all SQL statements executed so far in the transaction.
    pub(crate) statements: Vec<SqlStatementTrace>,
    /// All locks taken so far in the transaction.
    all_locks: HashSet<Lock>,
    /// The time the trace started
    pub(crate) trace_start: DateTime<Local>,
    /// All columns in the database, along with their metadata
    columns: HashMap<ColumnIdentifier, ColumnMetadata>,
    /// All constraints in the database
    constraints: HashMap<Oid, Constraint>,
}

impl TxLockTracer {
    /// Trace a single SQL statement, recording the locks taken and the duration of the statement.
    pub fn trace_sql_statement(&mut self, tx: &mut Transaction, sql: &str) -> Result<()> {
        let start_time = Instant::now();
        let oid_vec = self.initial_objects.iter().copied().collect_vec();
        tx.execute(sql, &[])
            .map_err(|err| anyhow!("{err} while executing {}", sql.to_owned()))?;
        let duration = start_time.elapsed();
        let locks_taken = find_relevant_locks_in_current_transaction(tx, &self.initial_objects)?;
        let new_locks = find_new_locks(&self.all_locks, &locks_taken);
        self.all_locks.extend(locks_taken.iter().cloned());

        let columns = fetch_all_columns(tx, &oid_vec)?;
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

        let constraints = fetch_constraints(tx, &oid_vec)?;
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

        self.statements.push(SqlStatementTrace {
            sql: sql.to_string(),
            locks_taken: new_locks.into_iter().collect(),
            start_time,
            duration,
            added_columns,
            modified_columns,
            added_constraints,
            modified_constraints,
        });
        Ok(())
    }
    pub fn new(
        name: Option<String>,
        initial_objects: HashSet<Oid>,
        columns: HashMap<ColumnIdentifier, ColumnMetadata>,
        constraints: HashMap<Oid, Constraint>,
    ) -> Self {
        Self {
            name,
            initial_objects,
            statements: vec![],
            all_locks: HashSet::new(),
            trace_start: Local::now(),
            columns,
            constraints,
        }
    }
}

/// Fetch all non-system columns in the database
fn fetch_all_columns(
    tx: &mut Transaction,
    oids: &[Oid],
) -> Result<HashMap<ColumnIdentifier, ColumnMetadata>> {
    let sql = "SELECT
           a.attrelid as table_oid,
           a.attnum as attnum,
           a.attname as column_name,
           a.attnotnull as not_null,
           t.typname as type_name,
           a.atttypmod as typmod
         FROM pg_catalog.pg_attribute a
           JOIN pg_catalog.pg_type t ON a.atttypid = t.oid
           JOIN pg_catalog.pg_class c ON a.attrelid = c.oid
           JOIN pg_catalog.pg_namespace n ON c.relnamespace = n.oid
         WHERE n.nspname NOT IN ('pg_catalog', 'information_schema') AND c.oid = ANY($1)
         ";
    let rows = tx.query(sql, &[&oids]).map_err(|err| anyhow!("{err}"))?;
    rows.into_iter()
        .map(|row| {
            let table_oid: Oid = row.try_get(0)?;
            let attnum: i16 = row.try_get(1)?;
            let column_name: String = row.try_get(2)?;
            let not_null: bool = row.try_get(3)?;
            let type_name: String = row.try_get(4)?;
            let typmod: i32 = row.try_get(5)?;
            let max_len = if typmod > 0 {
                Some((typmod - 4) as u32)
            } else {
                None
            };
            let identifier = ColumnIdentifier {
                oid: table_oid,
                attnum: attnum as i32,
            };
            let metadata = ColumnMetadata {
                name: column_name,
                nullable: !not_null,
                typename: type_name,
                max_len,
            };
            Ok((identifier, metadata))
        })
        .collect()
}

/// Fetch all non-system constraints in the database
fn fetch_constraints(tx: &mut Transaction, oids: &[Oid]) -> Result<HashMap<Oid, Constraint>> {
    let sql = "SELECT
           n.nspname as schema_name,
           c.relname as table_name,
           con.oid as con_oid,
           con.conname as constraint_name,
           con.contype as constraint_type,
           con.convalidated as valid,
           pg_get_constraintdef(con.oid) as expression
         FROM pg_catalog.pg_constraint con
           JOIN pg_catalog.pg_class c ON con.conrelid = c.oid
           JOIN pg_catalog.pg_namespace n ON c.relnamespace = n.oid
         WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
          AND con.conrelid = ANY($1) OR con.confrelid = ANY($1)
         ";
    let rows = tx.query(sql, &[&oids]).map_err(|err| anyhow!("{err}"))?;

    rows.into_iter()
        .map(|row| {
            let schema_name: String = row.try_get(0)?;
            let table_name: String = row.try_get(1)?;
            let con_oid: Oid = row.try_get(2)?;
            let constraint_name: String = row.try_get(3)?;
            let constraint_type_byte: i8 = row.try_get(4)?;
            let constraint_type = Contype::from_char((constraint_type_byte as u8) as char)?;
            let valid: bool = row.try_get(5)?;
            let expression: Option<String> = row.try_get(6)?;
            let constraint = Constraint {
                schema_name,
                table_name,
                constraint_type,
                name: constraint_name,
                expression,
                valid,
            };
            Ok((con_oid, constraint))
        })
        .collect()
}

/// Fetch all user owned lockable objects in the database, skipping the system schemas.
fn fetch_lockable_objects(tx: &mut Transaction) -> Result<HashSet<LockableTarget>, anyhow::Error> {
    let sql = "SELECT
           n.nspname as schema_name,
           c.relname as table_name,
           c.relkind as relkind,
           c.oid as oid
         FROM pg_catalog.pg_class c
           JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
         ";
    let rows = tx.query(sql, &[]).map_err(|err| anyhow!("{err}"))?;
    rows.into_iter()
        .map(|row| {
            let schema: String = row.try_get(0)?;
            let object_name: String = row.try_get(1)?;
            let rk_byte: i8 = row.try_get(2)?;
            let rel_kind: char = (rk_byte as u8) as char;
            let oid: Oid = row.try_get(3)?;
            LockableTarget::new(schema.as_str(), object_name.as_str(), rel_kind, oid).ok_or(
                anyhow!("{schema}.{object_name} has invalid relkind: {rel_kind}"),
            )
        })
        .collect()
}

/// Trace a transaction, executing a series of SQL statements and recording the locks taken.
pub fn trace_transaction<S: AsRef<str>>(
    name: Option<String>,
    tx: &mut Transaction,
    sql_statements: impl Iterator<Item = S>,
) -> Result<TxLockTracer> {
    let initial_objects: HashSet<_> = fetch_lockable_objects(tx)?
        .into_iter()
        .map(|obj| obj.oid)
        .collect();
    let oid_vec: Vec<_> = initial_objects.iter().copied().collect();
    let columns = fetch_all_columns(tx, &oid_vec)?;
    let constraints = fetch_constraints(tx, &oid_vec)?;
    let mut trace = TxLockTracer::new(name, initial_objects, columns, constraints);
    for sql in sql_statements {
        trace.trace_sql_statement(tx, sql.as_ref().trim())?;
    }
    Ok(trace)
}

#[cfg(test)]
mod tests {
    use postgres::{Client, NoTls};

    fn get_client() -> Client {
        Client::connect(
            "host=localhost dbname=test_db password=postgres user=postgres",
            NoTls,
        )
        .unwrap()
    }

    #[test]
    fn test_that_we_discover_modified_nullability() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books alter column title set not null"].into_iter(),
        )
        .unwrap();
        let modification = &trace.statements[0].modified_columns[0].1;
        assert!(modification.old.nullable);
        assert!(!modification.new.nullable);
    }

    #[test]
    fn test_that_we_discover_new_valid_check_constraint() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books add constraint check_title check (title <> '')"].into_iter(),
        )
        .unwrap();
        let constraint = &trace.statements[0].added_constraints[0];
        assert_eq!(constraint.constraint_type, super::Contype::Check);
        assert!(constraint.valid);
        assert_eq!(
            constraint.expression.clone().unwrap().as_str(),
            "CHECK ((title <> ''::text))"
        );
    }

    #[test]
    fn test_that_we_discover_new_foreign_key_constraint() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None, &mut tx, vec![
                "create table authors (id serial primary key);",
                "alter table books add column author_id integer;",
                "alter table books add constraint fk_author foreign key (author_id) references authors(id)",
            ].into_iter(),
        ).unwrap();
        let constraint = &trace.statements[2].added_constraints[0];
        assert_eq!(constraint.constraint_type, super::Contype::ForeignKey);
        assert!(constraint.valid);
        assert_eq!(
            constraint.expression.clone().unwrap().as_str(),
            "FOREIGN KEY (author_id) REFERENCES authors(id)"
        );
    }

    #[test]
    fn test_that_we_discover_new_not_valid_check_constraint() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books add constraint check_title check (title <> '') not valid"]
                .into_iter(),
        )
        .unwrap();
        let constraint = &trace.statements[0].added_constraints[0];
        assert_eq!(constraint.constraint_type, super::Contype::Check);
        assert!(!constraint.valid);
    }

    #[test]
    fn test_that_we_discover_column_renames() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books rename column title to book_title"].into_iter(),
        )
        .unwrap();
        let modification = &trace.statements[0].modified_columns[0].1;
        assert_eq!(modification.old.name, "title");
        assert_eq!(modification.new.name, "book_title");
    }

    #[test]
    fn test_that_we_discover_column_type_changes() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books alter column title type varchar(255)"].into_iter(),
        )
        .unwrap();
        let modification = &trace.statements[0].modified_columns[0].1;
        assert_eq!(modification.old.typename, "text");
        assert_eq!(modification.new.typename, "varchar");
        assert_eq!(modification.new.max_len.unwrap(), 255);
    }
}
