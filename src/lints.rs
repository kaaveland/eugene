use itertools::Itertools;

use crate::comments::filter_rules;
pub use crate::lints::ast::StatementSummary;
use crate::output::output_format::{LintReport, LintedStatement};

/// The `ast` module provides a way to describe a parsed SQL statement in a structured way,
/// using simpler trees than the ones provided by `pg_query`.
pub mod ast;
/// The `rules` module contains lint rules that can be matched to `LintedStatement`
pub mod rules;

/// Represents mutable state for linting through a single SQL script.
///
/// This struct is used to keep track of new objects, so the lint rules can check
/// visibility to other transactions, usage of lock timeouts and other properties
/// that require state to be kept between different statements.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct TransactionState {
    locktimeout: bool,
    created_objects: Vec<(String, String)>,
    altered_tables: Vec<(String, String)>,
    has_access_exclusive: bool,
}

impl TransactionState {
    /// Query if the script under linting has previously created an object with the given schema and name.
    pub fn has_created_object(&self, schema: &str, name: &str) -> bool {
        self.created_objects
            .iter()
            .any(|(s, n)| schema.eq_ignore_ascii_case(s) && name.eq_ignore_ascii_case(n))
    }
    /// Query if the script under linting has previously set a lock timeout.
    pub fn has_locktimeout(&self) -> bool {
        self.locktimeout
    }
    /// Update the context with the information from a new statement, logging new objects and lock timeouts.
    pub fn update_from(&mut self, summary: &StatementSummary) {
        if let StatementSummary::LockTimeout = summary {
            self.locktimeout = true;
        }
        summary.created_objects().iter().for_each(|(schema, name)| {
            self.created_objects
                .push((schema.to_string(), name.to_string()))
        });
        match summary {
            StatementSummary::AlterTable { schema, name, .. }
                if !self.has_created_object(schema, name) =>
            {
                self.has_access_exclusive = true;
            }
            _ => {}
        }

        if let StatementSummary::AlterTable { schema, name, .. } = summary {
            let new_item = (schema.to_string(), name.to_string());
            if !self.altered_tables.contains(&new_item) {
                self.altered_tables.push(new_item);
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct LintContext<'a> {
    pub(crate) ctx: &'a TransactionState,
    pub(crate) statement: &'a StatementSummary,
}

impl<'a> LintContext<'a> {
    pub fn new(ctx: &'a TransactionState, statement: &'a StatementSummary) -> Self {
        LintContext { ctx, statement }
    }
    /// Locks taken by the statement that were not created in the same transaction.
    pub fn locks_visible_outside_tx(&self) -> Vec<(&str, &str)> {
        self.statement
            .lock_targets()
            .iter()
            .filter(|(schema, name)| !self.ctx.has_created_object(schema, name))
            .copied()
            .collect()
    }
    /// True if the statement takes a lock on the given schema and name.
    pub fn takes_lock(&self, target_schema: &str, target_name: &str) -> bool {
        self.statement
            .lock_targets()
            .iter()
            .contains(&(target_schema, target_name))
    }
    /// True if the transaction has set a lock timeout.
    pub fn has_lock_timeout(&self) -> bool {
        self.ctx.has_locktimeout()
    }
    /// True if the lock target was created in another transaction
    pub fn is_visible(&self, schema: &str, name: &str) -> bool {
        !self.ctx.has_created_object(schema, name)
    }
    pub fn holding_access_exclusive(&self) -> bool {
        self.ctx.has_access_exclusive
    }
    /// True if the transaction has previously altered this table
    pub fn has_altered_table(&self, schema: &str, name: &str) -> bool {
        self.ctx
            .altered_tables
            .iter()
            .any(|(s, n)| schema.eq_ignore_ascii_case(s) && name.eq_ignore_ascii_case(n))
    }
}

/// Lint a SQL script and return a report with all matched lints for each statement.
pub fn lint<S: AsRef<str>>(
    name: Option<String>,
    sql: S,
    ignored_lints: &[&str],
    skip_summary: bool,
) -> anyhow::Result<LintReport> {
    let statements = pg_query::split_with_parser(sql.as_ref())?;
    let mut ctx = TransactionState::default();
    let mut lints = Vec::new();
    let mut no: usize = 1;
    let mut line_number: usize = 1;
    let mut passed_all = true;
    for stmt in statements {
        let action = crate::comments::find_comment_action(sql.as_ref())?;
        let tree = pg_query::parse(stmt)?;
        for raw in tree.protobuf.stmts.iter() {
            if let Some(node) = &raw.stmt {
                if let Some(node_ref) = &node.node {
                    let summary = ast::describe(&node_ref.to_ref())?;
                    let lint_line = LintContext::new(&ctx, &summary);
                    let matched_lints: Vec<_> = filter_rules(&action, rules::all_rules())
                        .filter(|rule| !ignored_lints.contains(&rule.id()))
                        .filter_map(|rule| rule.check(lint_line))
                        .collect();
                    passed_all = passed_all && matched_lints.is_empty();

                    lints.push(LintedStatement {
                        statement_number: no,
                        line_number,
                        sql: stmt.trim().to_string(),
                        triggered_rules: matched_lints,
                    });
                    ctx.update_from(&summary);
                    no += 1;
                    line_number += stmt.lines().count();
                }
            }
        }
    }
    Ok(LintReport {
        name,
        statements: lints,
        passed_all_checks: passed_all,
        skip_summary,
    })
}

pub fn anon_lint<S: AsRef<str>>(sql: S) -> anyhow::Result<LintReport> {
    lint(None, sql, &[], false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hint_data::ADDED_SERIAL_OR_STORED_GENERATED_COLUMN;

    fn matched_lint_rule(report: &LintReport, rule_id: &str) -> bool {
        report
            .statements
            .iter()
            .any(|lint| lint.triggered_rules.iter().any(|hint| hint.id == rule_id))
    }

    #[test]
    fn test_no_locktimeout_create_index() {
        let report = anon_lint("create index books_title_idx on books(title);").unwrap();
        assert!(matched_lint_rule(&report, rules::LOCKTIMEOUT_WARNING.id()));
    }

    #[test]
    fn test_locktimeout_create_index_on_new_table() {
        let report = anon_lint(
            "create table books(id serial primary key, title text); \
            create index books_title_idx on books(title);",
        )
        .unwrap();
        assert!(!matched_lint_rule(&report, rules::LOCKTIMEOUT_WARNING.id()));
    }

    #[test]
    fn test_locktimeout_alter_table_without_timeout() {
        let report =
            anon_lint("alter table books add constraint check_price check (price > 0);").unwrap();
        assert!(matched_lint_rule(&report, rules::LOCKTIMEOUT_WARNING.id()));
    }

    #[test]
    fn test_locktimeout_alter_table_with_timeout() {
        let report =
            anon_lint("set lock_timeout = '2s'; create index books_title_idx on books(title);")
                .unwrap();
        assert!(!matched_lint_rule(&report, rules::LOCKTIMEOUT_WARNING.id()));
    }

    #[test]
    fn test_create_index_on_new_table() {
        let report = anon_lint(
            "create table books(id serial primary key, title text); \
            create index books_title_idx on books(title);",
        )
        .unwrap();
        assert!(!matched_lint_rule(
            &report,
            rules::CREATE_INDEX_NONCONCURRENTLY.id()
        ));
    }

    #[test]
    fn test_create_index_concurrently_is_not_dangerous_lock() {
        let report =
            anon_lint("create index concurrently books_title_idx on books(title);").unwrap();
        assert!(!matched_lint_rule(&report, rules::LOCKTIMEOUT_WARNING.id()));
    }

    #[test]
    fn test_create_index_on_existing_table() {
        let report = anon_lint("create index books_title_idx on books(title);").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::CREATE_INDEX_NONCONCURRENTLY.id()
        ));
    }

    #[test]
    fn test_add_check_constraint_to_existing_table() {
        let report =
            anon_lint("alter table books add constraint check_price check (price > 0);").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::ADDING_VALID_CONSTRAINT.id()
        ));
    }

    #[test]
    fn test_add_check_constraint_to_new_table() {
        let report = anon_lint(
            "create table books(id serial primary key, title text); \
            alter table books add constraint check_price check (price > 0);",
        )
        .unwrap();
        assert!(!matched_lint_rule(
            &report,
            rules::ADDING_VALID_CONSTRAINT.id()
        ));
    }

    #[test]
    fn test_add_not_valid_constraint_to_existing_table() {
        let report =
            anon_lint("alter table books add constraint check_price check (price > 0) not valid;")
                .unwrap();
        assert!(!matched_lint_rule(
            &report,
            rules::ADDING_VALID_CONSTRAINT.id()
        ));
    }

    #[test]
    fn test_adding_exclusion_constraint_to_existing_table() {
        let report =
            anon_lint("alter table books add constraint exclude_price exclude (price with =);")
                .unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::ADDING_EXCLUSION_CONSTRAINT.id()
        ));
    }

    #[test]
    fn test_adding_exclusion_constraint_on_new_table() {
        let report = anon_lint(
            "create table books(id serial primary key, title text);\
             alter table books add constraint exclude_price exclude (price with =);",
        )
        .unwrap();
        assert!(!matched_lint_rule(
            &report,
            rules::ADDING_EXCLUSION_CONSTRAINT.id()
        ));
    }

    #[test]
    fn test_adding_unique_constraint_using_idx() {
        let report = anon_lint(
            "alter table books add constraint unique_title unique using index unique_title_idx;",
        )
        .unwrap();
        assert!(!matched_lint_rule(
            &report,
            rules::ADD_NEW_UNIQUE_CONSTRAINT_WITHOUT_USING_INDEX.id()
        ));
    }

    #[test]
    fn test_adding_unique_constraint() {
        let report =
            anon_lint("alter table books add constraint unique_title unique (title);").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::ADD_NEW_UNIQUE_CONSTRAINT_WITHOUT_USING_INDEX.id()
        ));
    }

    #[test]
    fn test_adding_unique_constraint_on_new_table() {
        let report = anon_lint(
            "create table books(id serial primary key, title text);\
             alter table books add constraint unique_title unique (title);",
        )
        .unwrap();
        assert!(!matched_lint_rule(
            &report,
            rules::ADD_NEW_UNIQUE_CONSTRAINT_WITHOUT_USING_INDEX.id()
        ));
    }

    #[test]
    fn test_sets_column_to_not_null_on_visible_table() {
        let report = anon_lint("alter table books alter column title set not null;").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK.id()
        ));
    }

    #[test]
    fn test_sets_column_to_not_null_on_new_table() {
        let report = anon_lint(
            "create table books(id serial primary key, title text);\
             alter table books alter column title set not null;",
        )
        .unwrap();
        assert!(!matched_lint_rule(
            &report,
            rules::MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK.id()
        ));
    }

    #[test]
    fn test_adding_json_column() {
        let report = anon_lint("alter table books add column data json;").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::SET_COLUMN_TYPE_TO_JSON.id()
        ));
    }

    #[test]
    fn test_alter_to_json_type() {
        let report = anon_lint("alter table books alter column data type json;").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::SET_COLUMN_TYPE_TO_JSON.id()
        ));
    }

    #[test]
    fn test_sets_new_data_type_to_column() {
        let report = anon_lint("alter table books alter column data type jsonb;").unwrap();
        assert!(matched_lint_rule(&report, rules::CHANGE_COLUMN_TYPE.id()));
    }

    #[test]
    fn test_ignore_json_rule_id() {
        let id = rules::SET_COLUMN_TYPE_TO_JSON.id();
        let sql = format!("-- eugene: ignore {id}\nalter table books add column data json;");
        let report = anon_lint(sql).unwrap();
        assert!(!matched_lint_rule(&report, id));
    }

    #[test]
    fn test_creates_table_with_json_column() {
        let report = anon_lint("create table books(id serial primary key, data json);").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::SET_COLUMN_TYPE_TO_JSON.id()
        ));
    }

    #[test]
    fn test_alter_table_add_serial_column() {
        let report = anon_lint("alter table books add column id serial;").unwrap();
        assert!(matched_lint_rule(
            &report,
            ADDED_SERIAL_OR_STORED_GENERATED_COLUMN.id
        ));
        let report = anon_lint("alter table books add column id bigserial;").unwrap();
        assert!(matched_lint_rule(
            &report,
            ADDED_SERIAL_OR_STORED_GENERATED_COLUMN.id
        ));
    }

    #[test]
    fn test_alter_table_generated_always_column() {
        let report =
            anon_lint("alter table books add column id int generated always as identity;").unwrap();
        assert!(!matched_lint_rule(
            &report,
            ADDED_SERIAL_OR_STORED_GENERATED_COLUMN.id
        ));
    }

    #[test]
    fn test_alter_table_stored_generated_column() {
        let report = anon_lint(
            "alter table books add column id int generated always as (1 + old_id) stored;",
        )
        .unwrap();
        assert!(matched_lint_rule(
            &report,
            ADDED_SERIAL_OR_STORED_GENERATED_COLUMN.id
        ));
    }

    #[test]
    fn test_altered_table_several_times() {
        let report = anon_lint(
            "
             alter table books add column data jsonb;\
             alter table books add column price numeric;
        ",
        )
        .unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::MULTIPLE_ALTER_TABLES_WHERE_ONE_WILL_DO.id()
        ));
    }

    #[test]
    fn test_altered_table_once_with_multiple_statements() {
        let report = anon_lint(
            "
             alter table books add column data jsonb, add column price numeric;
        ",
        )
        .unwrap();
        assert!(!matched_lint_rule(
            &report,
            rules::MULTIPLE_ALTER_TABLES_WHERE_ONE_WILL_DO.id()
        ));
    }

    #[test]
    fn test_create_an_enum() {
        let report = anon_lint("create type mood as enum ('happy', 'sad');").unwrap();
        assert!(matched_lint_rule(&report, rules::CREATING_ENUM.id()));
    }

    #[test]
    fn test_add_pk_using_index() {
        let report =
            anon_lint("alter table books add primary key using index books_pkey;").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::ADD_PRIMARY_KEY_USING_INDEX.id()
        ));
    }

    #[test]
    fn test_add_pk_with_using_index() {
        let sql = "alter table books add primary key (id);";
        let report = anon_lint(sql).unwrap();
        assert!(!matched_lint_rule(
            &report,
            rules::ADD_PRIMARY_KEY_USING_INDEX.id()
        ));
        // Covered by another lint
        assert!(matched_lint_rule(
            &report,
            rules::ADD_NEW_UNIQUE_CONSTRAINT_WITHOUT_USING_INDEX.id()
        ));
    }
}
