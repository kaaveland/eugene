use postgres::Transaction;
use std::collections::HashSet;
pub use tracer::{SqlStatementTrace, TxLockTracer};
pub mod queries;
/// Implementation details of the lock tracer.
pub mod tracer;

/// Trace a transaction, executing a series of SQL statements and recording the locks taken.
pub fn trace_transaction<'a, S: AsRef<str>>(
    name: Option<String>,
    tx: &mut Transaction,
    sql_statements: impl Iterator<Item = S>,
    ignored_hints: &'a [&'a str],
) -> anyhow::Result<TxLockTracer<'a>> {
    let initial_objects: HashSet<_> = queries::fetch_lockable_objects(tx, &[])?
        .into_iter()
        .map(|obj| obj.oid)
        .collect();
    let oid_vec: Vec<_> = initial_objects.iter().copied().collect();
    let columns = queries::fetch_all_columns(tx, &oid_vec)?;
    let constraints = queries::fetch_constraints(tx, &oid_vec)?;
    let relfile_ids = queries::fetch_all_rel_file_ids(tx, &oid_vec)?
        .into_iter()
        .map(|(oid, relfile_id)| (oid, relfile_id.relfilenode))
        .collect();
    let mut trace = TxLockTracer::new(
        name,
        initial_objects,
        columns,
        constraints,
        relfile_ids,
        ignored_hints,
    );
    for sql in sql_statements {
        trace.trace_sql_statement(tx, sql.as_ref().trim())?;
    }
    Ok(trace)
}

#[cfg(test)]
mod tests {
    use postgres::{Client, NoTls};

    use crate::generate_new_test_db;
    use crate::hint_data;
    use crate::pg_types::contype::Contype;
    use crate::pg_types::lock_modes::LockMode;
    use pretty_assertions::assert_eq;

    fn get_client() -> Client {
        let test_db = generate_new_test_db();
        Client::connect(
            format!("host=localhost dbname={test_db} password=postgres user=postgres").as_str(),
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
            &[],
        )
        .unwrap();
        let modification = &trace.statements[0].modified_columns[0].1;
        assert!(modification.old.nullable);
        assert!(!modification.new.nullable);
        assert!(trace.triggered_hints[0]
            .iter()
            .any(|hint| hint.id == hint_data::MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK.id));
    }

    #[test]
    fn test_that_we_discover_new_valid_check_constraint() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books add constraint check_title check (title <> '')"].into_iter(),
            &[],
        )
        .unwrap();
        let constraint = &trace.statements[0].added_constraints[0];
        assert_eq!(constraint.constraint_type, Contype::Check);
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
            &[]
        ).unwrap();
        let constraint = &trace.statements[2].added_constraints[0];
        assert_eq!(constraint.constraint_type, Contype::ForeignKey);
        assert!(constraint.valid);
        assert_eq!(
            constraint.expression.clone().unwrap().as_str(),
            "FOREIGN KEY (author_id) REFERENCES authors(id)"
        );
        assert!(trace.triggered_hints[2]
            .iter()
            .any(|hint| hint.id == hint_data::VALIDATE_CONSTRAINT_WITH_LOCK.id));
        assert!(trace.triggered_hints[2]
            .iter()
            .any(|hint| hint.id == hint_data::TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT.id));
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
            &[],
        )
        .unwrap();
        let constraint = &trace.statements[0].added_constraints[0];
        assert_eq!(constraint.constraint_type, Contype::Check);
        assert!(!constraint.valid);
        assert!(!trace.triggered_hints[0]
            .iter()
            .any(|hint| hint.id == hint_data::VALIDATE_CONSTRAINT_WITH_LOCK.id));
    }

    #[test]
    fn test_that_we_discover_column_renames() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books rename column title to book_title"].into_iter(),
            &[],
        )
        .unwrap();
        let modification = &trace.statements[0].modified_columns[0].1;
        assert_eq!(modification.old.column_name, "title");
        assert_eq!(modification.new.column_name, "book_title");
    }

    #[test]
    fn test_that_we_discover_column_type_changes() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books alter column title type varchar(255)"].into_iter(),
            &[],
        )
        .unwrap();
        let modification = &trace.statements[0].modified_columns[0].1;
        assert_eq!(modification.old.typename, "text");
        assert_eq!(modification.new.typename, "varchar");
        assert_eq!(modification.new.max_len.unwrap(), 255);
        assert!(trace.triggered_hints[0]
            .iter()
            .any(|hint| hint.id == hint_data::TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT.id));
        assert!(trace.triggered_hints[0]
            .iter()
            .any(|hint| hint.id == hint_data::TYPE_CHANGE_REQUIRES_TABLE_REWRITE.id));
    }

    #[test]
    fn test_that_we_see_new_access_share_lock() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace =
            super::trace_transaction(None, &mut tx, vec!["select * from books"].into_iter(), &[])
                .unwrap();
        let lock = &trace.statements[0].locks_taken[0];
        assert_eq!(lock.mode, LockMode::AccessShare);
        let is_pkey = lock.target.rel_kind.is_index();
        if is_pkey {
            assert_eq!(lock.target.object_name, "books_pkey");
        } else {
            assert_eq!(lock.target.object_name, "books");
        }
    }

    #[test]
    fn test_that_we_see_access_exclusive_lock_on_alter() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books add column metadata text"].into_iter(),
            &[],
        )
        .unwrap();
        let lock = trace
            .all_locks
            .iter()
            .find(|lock| lock.mode == LockMode::AccessExclusive)
            .unwrap();

        assert_eq!(lock.target.object_name, "books");
    }

    #[test]
    fn test_creating_index_blocks_writes() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["create index on books (title)"].into_iter(),
            &[],
        )
        .unwrap();
        let lock = trace
            .all_locks
            .iter()
            .find(|lock| lock.mode.blocked_queries().contains(&"INSERT"));

        assert!(lock.is_some());
    }

    #[test]
    fn discovers_new_index() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["create index on books (title)"].into_iter(),
            &[],
        )
        .unwrap();

        assert!(trace.statements[0]
            .created_objects
            .iter()
            .any(|obj| obj.object_name == "books_title_idx"));
        assert!(trace.triggered_hints[0]
            .iter()
            .any(|hint| hint.id == hint_data::NEW_INDEX_ON_EXISTING_TABLE_IS_NONCONCURRENT.id));
        assert!(trace.triggered_hints[0]
            .iter()
            .any(|hint| hint.id == hint_data::TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT.id));
    }

    #[test]
    fn ignores_new_index_on_new_table() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec![
                "create table papers (id serial primary key, title text not null);",
                "create index papers_title_idx on papers (title)",
            ]
            .into_iter(),
            &[],
        )
        .unwrap();
        assert!(trace.triggered_hints[0].is_empty());
        assert!(trace.triggered_hints[1].is_empty());
        assert!(trace.statements[1].locks_taken.is_empty());
    }

    #[test]
    fn add_unique_constraint_using_unique_index_is_safe() {
        let mut client = get_client();
        client
            .execute("create unique index books_title_uq on books(title);", &[])
            .unwrap();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books add constraint unique_title unique using index books_title_uq"]
                .into_iter(),
            &[],
        )
        .unwrap();
        assert!(trace.statements[0].created_objects.is_empty());
        assert!(trace.triggered_hints[0]
            .iter()
            .any(|hint| hint.id == hint_data::TOOK_DANGEROUS_LOCK_WITHOUT_TIMEOUT.id));
    }

    #[test]
    fn discovers_lock_timeout_from_set() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec![
                "set lock_timeout = 1000",
                "alter table books add column metadata text",
            ]
            .into_iter(),
            &[],
        )
        .unwrap();
        assert_eq!(trace.statements[1].lock_timeout_millis, 1000);
        assert!(trace.triggered_hints[0].is_empty());
        assert!(trace.triggered_hints[1].is_empty());
    }

    #[test]
    fn test_that_we_stop_json() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books add column metadata json"].into_iter(),
            &[],
        )
        .unwrap();
        let modification = &trace.statements[0].added_columns[0].1;
        assert_eq!(modification.typename, "json");
        assert!(trace.triggered_hints[0]
            .iter()
            .any(|hint| hint.id == hint_data::ADD_JSON_COLUMN.id));
    }

    #[test]
    fn test_that_we_discover_valid_check_not_null_when_modifying_to_null() {
        let mut client = get_client();
        client
            .execute(
                "alter table books add constraint check_title check (title is not null)",
                &[],
            )
            .unwrap();

        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books alter column title set not null"].into_iter(),
            &[],
        )
        .unwrap();
        let modification = &trace.statements[0].modified_columns[0].1;
        assert!(!modification.new.nullable);
        assert!(!trace.triggered_hints[0]
            .iter()
            .any(|hint| hint.id == hint_data::MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK.id));
    }

    #[test]
    fn test_widening_type_causes_rewrite() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books alter column price type bigint"].into_iter(),
            &[],
        )
        .unwrap();
        assert!(trace.statements[0]
            .rewritten_objects
            .iter()
            .any(|obj| obj.object_name == "books" && obj.schema_name == "public"));
    }

    #[test]
    fn test_dropping_column_does_not_cause_rewrite() {
        let mut client = get_client();
        client
            .execute("insert into books (title) values ('hello')", &[])
            .unwrap();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["alter table books drop column title"].into_iter(),
            &[],
        )
        .unwrap();
        assert!(trace.statements[0].rewritten_objects.is_empty());
    }

    #[test]
    fn test_ignore_all_triggers_no_hints() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec!["-- eugene: ignore\nalter table books add column meta json;"].into_iter(),
            &[],
        )
        .unwrap();
        assert!(trace.triggered_hints[0].is_empty());
    }

    #[test]
    fn test_ignore_specific_hint_triggers_other_hints() {
        let mut client = get_client();
        let mut tx = client.transaction().unwrap();
        let json_id = hint_data::ADD_JSON_COLUMN.id;
        let trace = super::trace_transaction(
            None,
            &mut tx,
            vec![&format!(
                "-- eugene: ignore {json_id}\nalter table books add column meta json;"
            )]
            .into_iter(),
            &[],
        )
        .unwrap();
        assert!(!trace.triggered_hints[0]
            .iter()
            .any(|hint| hint.id == hint_data::ADD_JSON_COLUMN.id));
        assert!(!trace.triggered_hints.is_empty())
    }
}
