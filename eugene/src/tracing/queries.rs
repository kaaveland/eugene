use crate::error::{ContextualResult, InnerError};
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use postgres::types::Oid;
use postgres::Transaction;

use crate::pg_types::contype::Contype;
use crate::pg_types::locks::{InvalidLockError, Lock, LockableTarget};
use crate::pg_types::relkinds::RelKind;

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub struct ColumnIdentifier {
    pub(crate) oid: Oid,
    pub(crate) attnum: i32,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ColumnMetadata {
    pub(crate) schema_name: String,
    pub(crate) table_name: String,
    pub(crate) column_name: String,
    pub(crate) nullable: bool,
    pub(crate) typename: String,
    pub(crate) max_len: Option<u32>,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Constraint {
    pub(crate) schema_name: String,
    pub(crate) table_name: String,
    pub(crate) constraint_type: Contype,
    pub(crate) name: String,
    pub(crate) expression: Option<String>,
    pub(crate) valid: bool,
    pub(crate) target: Oid,
    pub(crate) fk_target: Option<Oid>
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct RelfileId {
    pub(crate) schema_name: String,
    pub(crate) object_name: String,
    pub(crate) relfilenode: u32,
    pub(crate) rel_kind: RelKind,
    pub(crate) oid: Oid,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ForeignKeyReference {
    pub(crate) constraint_name: String,
    pub(crate) schema_name: String,
    pub(crate) table_name: String,
    pub(crate) columns: Vec<String>,
}

/// Enumerate all locks owned by the current transaction.
fn query_pg_locks_in_current_transaction(tx: &mut Transaction) -> crate::Result<HashSet<Lock>> {
    let query = "SELECT n.nspname::text AS schema_name,
                c.relname::text AS object_name,
                c.relkind AS relkind,
                l.mode::text AS mode,
                c.oid AS oid
         FROM pg_locks l JOIN pg_class c ON c.oid = l.relation
           JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE l.locktype = 'relation' AND l.pid = pg_backend_pid();";
    let rows = tx
        .query(query, &[])
        .with_context("failed to query pg_locks_in_current_transaction")?;
    let locks = rows
        .into_iter()
        .map(|row| {
            let schema: String = row.try_get(0)?;
            let object_name: String = row.try_get(1)?;
            let relkind: i8 = row.try_get(2)?;
            let mode: String = row.try_get(3)?;
            let oid: Oid = row.try_get(4)?;
            Lock::new(schema, object_name, mode, (relkind as u8) as char, oid).map_err(|e| e.into())
        })
        .collect::<crate::Result<HashSet<Lock>>>()?;
    Ok(locks)
}

/// Find all locks in the current transaction that are relevant to the given set of objects.
pub fn find_relevant_locks_in_current_transaction(
    tx: &mut Transaction,
    relevant_objects: &HashSet<Oid>,
) -> crate::Result<HashSet<Lock>> {
    let current_locks = query_pg_locks_in_current_transaction(tx)?;
    Ok(current_locks
        .into_iter()
        .filter(|lock| relevant_objects.contains(&lock.target_oid()))
        .collect())
}

/// Return the locks that are new in the new set of locks compared to the old set.
pub fn find_new_locks(old_locks: &HashSet<Lock>, new_locks: &HashSet<Lock>) -> HashSet<Lock> {
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

/// Fetch all non-system columns in the database
pub fn fetch_all_columns(
    tx: &mut Transaction,
    oids: &[Oid],
) -> crate::Result<HashMap<ColumnIdentifier, ColumnMetadata>> {
    let sql = "SELECT
           a.attrelid as table_oid,
           a.attnum as attnum,
           a.attname as column_name,
           a.attnotnull as not_null,
           t.typname as type_name,
           a.atttypmod as typmod,
           n.nspname as schema_name,
           c.relname as table_name
         FROM pg_catalog.pg_attribute a
           JOIN pg_catalog.pg_type t ON a.atttypid = t.oid
           JOIN pg_catalog.pg_class c ON a.attrelid = c.oid
           JOIN pg_catalog.pg_namespace n ON c.relnamespace = n.oid
         WHERE n.nspname NOT IN ('pg_catalog', 'information_schema') AND c.oid = ANY($1)
         ";
    let rows = tx
        .query(sql, &[&oids])
        .with_context("failed to fetch all columns")?;
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
            let schema_name: String = row.try_get(6)?;
            let table_name: String = row.try_get(7)?;
            let identifier = ColumnIdentifier {
                oid: table_oid,
                attnum: attnum as i32,
            };
            let metadata = ColumnMetadata {
                column_name,
                nullable: !not_null,
                typename: type_name,
                max_len,
                schema_name,
                table_name,
            };
            Ok((identifier, metadata))
        })
        .collect()
}

/// Fetch all non-system constraints in the database that match an `oid`
pub fn fetch_constraints(
    tx: &mut Transaction,
    oids: &[Oid],
) -> crate::Result<HashMap<Oid, Constraint>> {
    let sql = "SELECT
           n.nspname as schema_name,
           c.relname as table_name,
           con.oid as con_oid,
           con.conname as constraint_name,
           con.contype as constraint_type,
           con.convalidated as valid,
           pg_get_constraintdef(con.oid) as expression,
           con.conrelid as target,
           con.confrelid as fk_target
         FROM pg_catalog.pg_constraint con
           JOIN pg_catalog.pg_class c ON con.conrelid = c.oid
           JOIN pg_catalog.pg_namespace n ON c.relnamespace = n.oid
         WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
          AND con.conrelid = ANY($1) OR con.confrelid = ANY($1)
         ";
    let rows = tx
        .query(sql, &[&oids])
        .with_context("failed to fetch all constraints")?;

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
            let target: Oid = row.try_get(7)?;
            let fk_target: Option<Oid> = row.try_get(8)?;
            let constraint = Constraint {
                schema_name,
                table_name,
                constraint_type,
                name: constraint_name,
                expression,
                valid,
                target,
                fk_target,
            };
            Ok((con_oid, constraint))
        })
        .collect()
}

/// Fetch all user owned lockable objects in the database, skipping the system schemas and objects in `skip_list`
pub fn fetch_lockable_objects(
    tx: &mut Transaction,
    skip_list: &[Oid],
) -> crate::Result<HashSet<LockableTarget>> {
    let sql = "SELECT
           n.nspname as schema_name,
           c.relname as table_name,
           c.relkind as relkind,
           c.oid as oid
         FROM pg_catalog.pg_class c
           JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
         WHERE
           n.nspname NOT IN ('pg_catalog', 'information_schema') AND NOT c.oid = ANY($1)
         ";
    let rows = tx
        .query(sql, &[&skip_list])
        .with_context("failed to fetch lockable objects")?;

    rows.into_iter()
        .map(|row| {
            let schema: String = row.try_get(0)?;
            let object_name: String = row.try_get(1)?;
            let rk_byte: i8 = row.try_get(2)?;
            let rel_kind: char = (rk_byte as u8) as char;
            let oid: Oid = row.try_get(3)?;
            LockableTarget::new(schema.as_str(), object_name.as_str(), rel_kind, oid)
                .ok_or_else(|| InvalidLockError::InvalidRelKind(rel_kind).into())
        })
        .collect()
}

/// Fetch all non-system relation file ids in the database
pub fn fetch_all_rel_file_ids(
    tx: &mut Transaction,
    tracked_objects: &[Oid],
) -> crate::Result<HashMap<Oid, RelfileId>> {
    // select schema, name, relfilenode, oid from pg_class where oid = any($1)
    let query = "SELECT c.oid, c.relfilenode, n.nspname, c.relname, c.relkind
         FROM pg_catalog.pg_class c
           JOIN pg_catalog.pg_namespace n ON c.relnamespace = n.oid
         WHERE c.oid = ANY($1)";
    let rows = tx.query(query, &[&tracked_objects])?;
    rows.into_iter()
        .map(|row| {
            let oid: Oid = row.try_get(0)?;
            let relfilenode: u32 = row.try_get(1)?;
            let schema_name: String = row.try_get(2)?;
            let table_name: String = row.try_get(3)?;
            let relkind = row.try_get::<_, i8>(4)?;
            let relkind = (relkind as u8) as char;
            let relkind =
                RelKind::from_db_code(relkind).ok_or(InvalidLockError::InvalidRelKind(relkind))?;
            Ok((
                oid,
                RelfileId {
                    schema_name,
                    object_name: table_name,
                    relfilenode,
                    oid,
                    rel_kind: relkind,
                },
            ))
        })
        .collect()
}

/// Retrieve the current `lock_timeout` for the active transaction
pub fn get_lock_timeout(tx: &mut Transaction) -> crate::Result<u64> {
    let query = "select current_setting('lock_timeout')";
    let timeout: String = tx
        .query_one(query, &[])
        .with_context("get lock timeout failed")?
        .try_get(0)
        .with_context("read lock timeout string")?;
    let digits = timeout
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    let unit = timeout
        .chars()
        .skip_while(|c| c.is_ascii_digit())
        .collect::<String>();
    let n: u64 = digits.parse()?;
    match unit.as_str() {
        "ms" | "" => Ok(n),
        "s" => Ok(n * 1000),
        "min" => Ok(n * 60 * 1000),
        "h" => Ok(n * 60 * 60 * 1000),
        "d" => Ok(n * 24 * 60 * 60 * 1000),
        _ => Err(InnerError::InvalidUnit(unit).into()),
    }
}

