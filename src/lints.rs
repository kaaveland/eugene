use anyhow::anyhow;
use itertools::Itertools;

pub use crate::lints::ast::StatementSummary;
use crate::output::output_format::{Lint, LintReport};

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
pub struct LintContext {
    locktimeout: bool,
    created_objects: Vec<(String, String)>,
    has_access_exclusive: bool,
}

impl LintContext {
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
    }
}

#[derive(Copy, Clone)]
pub struct LintedStatement<'a> {
    pub(crate) ctx: &'a LintContext,
    pub(crate) statement: &'a StatementSummary,
}

impl<'a> LintedStatement<'a> {
    pub fn new(ctx: &'a LintContext, statement: &'a StatementSummary) -> Self {
        LintedStatement { ctx, statement }
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
}

enum LintAction<'a> {
    SkipAll,
    Skip(Vec<&'a str>),
    Continue,
}

/// Lint a SQL script and return a report with all matched lints for each statement.
pub fn lint<S: AsRef<str>>(sql: S) -> anyhow::Result<LintReport> {
    let statements = pg_query::split_with_parser(sql.as_ref())?;
    let eugene_comment_regex = regex::Regex::new(r"-- eugene: ([^\n]+)")?;
    let mut ctx = LintContext::default();
    let mut lints = Vec::new();
    let mut no: usize = 1;
    for stmt in statements {
        let m = eugene_comment_regex.find(stmt);
        let action: anyhow::Result<_> = if let Some(eugene_instruction) = m {
            match eugene_instruction.as_str() {
                "ignore" => Ok(LintAction::SkipAll),
                ids if ids.starts_with("ignore ") => {
                    let rem = &ids["ignore ".len()..];
                    Ok(LintAction::Skip(rem.split(',').collect()))
                }
                _ => Err(anyhow!(
                    "Invalid eugene instruction: {}",
                    eugene_instruction.as_str()
                ))?,
            }
        } else {
            Ok(LintAction::Continue)
        };
        let action = action?;
        let tree = pg_query::parse(stmt)?;
        for raw in tree.protobuf.stmts.iter() {
            if let Some(node) = &raw.stmt {
                if let Some(node_ref) = &node.node {
                    let summary = ast::describe(&node_ref.to_ref())?;
                    let lint_line = LintedStatement::new(&ctx, &summary);

                    let matched_lints = if matches!(action, LintAction::SkipAll) {
                        vec![]
                    } else {
                        rules::all_rules()
                            .filter_map(|rule| rule.check(lint_line))
                            .filter(|hint| {
                                if let LintAction::Skip(ids) = &action {
                                    !ids.contains(&hint.id.as_str())
                                } else {
                                    true
                                }
                            })
                            .collect()
                    };
                    lints.push(Lint {
                        statement_number: no,
                        sql: stmt.trim().to_string(),
                        lints: matched_lints,
                    });
                    ctx.update_from(&summary);
                    no += 1;
                }
            }
        }
    }
    Ok(LintReport { lints })
}

/// Skip the ignored lint IDs
pub fn apply_ignore_list(report: &LintReport, ignored_hints: &[String]) -> LintReport {
    let lints = report
        .lints
        .iter()
        .map(|stmt| Lint {
            lints: stmt
                .lints
                .iter()
                .filter(|hint| !ignored_hints.contains(&hint.id))
                .cloned()
                .collect(),
            ..stmt.clone()
        })
        .collect();
    LintReport { lints }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matched_lint_rule(report: &LintReport, rule_id: &str) -> bool {
        report
            .lints
            .iter()
            .any(|lint| lint.lints.iter().any(|hint| hint.id == rule_id))
    }

    #[test]
    fn test_no_locktimeout_create_index() {
        let report = lint("create index books_title_idx on books(title);").unwrap();
        assert!(matched_lint_rule(&report, rules::LOCKTIMEOUT_WARNING.id()));
    }

    #[test]
    fn test_locktimeout_create_index_on_new_table() {
        let report = lint(
            "create table books(id serial primary key, title text); \
            create index books_title_idx on books(title);",
        )
        .unwrap();
        assert!(!matched_lint_rule(&report, rules::LOCKTIMEOUT_WARNING.id()));
    }

    #[test]
    fn test_locktimeout_alter_table_without_timeout() {
        let report =
            lint("alter table books add constraint check_price check (price > 0);").unwrap();
        assert!(matched_lint_rule(&report, rules::LOCKTIMEOUT_WARNING.id()));
    }

    #[test]
    fn test_locktimeout_alter_table_with_timeout() {
        let report =
            lint("set lock_timeout = '2s'; create index books_title_idx on books(title);").unwrap();
        assert!(!matched_lint_rule(&report, rules::LOCKTIMEOUT_WARNING.id()));
    }

    #[test]
    fn test_create_index_on_new_table() {
        let report = lint(
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
    fn test_create_index_on_existing_table() {
        let report = lint("create index books_title_idx on books(title);").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::CREATE_INDEX_NONCONCURRENTLY.id()
        ));
    }

    #[test]
    fn test_add_check_constraint_to_existing_table() {
        let report =
            lint("alter table books add constraint check_price check (price > 0);").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::ADDING_VALID_CONSTRAINT.id()
        ));
    }

    #[test]
    fn test_add_check_constraint_to_new_table() {
        let report = lint(
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
            lint("alter table books add constraint check_price check (price > 0) not valid;")
                .unwrap();
        assert!(!matched_lint_rule(
            &report,
            rules::ADDING_VALID_CONSTRAINT.id()
        ));
    }

    #[test]
    fn test_adding_exclusion_constraint_to_existing_table() {
        let report =
            lint("alter table books add constraint exclude_price exclude (price with =);").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::ADDING_EXCLUSION_CONSTRAINT.id()
        ));
    }

    #[test]
    fn test_adding_exclusion_constraint_on_new_table() {
        let report = lint(
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
        let report = lint(
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
        let report = lint("alter table books add constraint unique_title unique (title);").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::ADD_NEW_UNIQUE_CONSTRAINT_WITHOUT_USING_INDEX.id()
        ));
    }

    #[test]
    fn test_adding_unique_constraint_on_new_table() {
        let report = lint(
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
        let report = lint("alter table books alter column title set not null;").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::MAKE_COLUMN_NOT_NULLABLE_WITH_LOCK.id()
        ));
    }

    #[test]
    fn test_sets_column_to_not_null_on_new_table() {
        let report = lint(
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
        let report = lint("alter table books add column data json;").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::SET_COLUMN_TYPE_TO_JSON.id()
        ));
    }

    #[test]
    fn test_alter_to_json_type() {
        let report = lint("alter table books alter column data type json;").unwrap();
        assert!(matched_lint_rule(
            &report,
            rules::SET_COLUMN_TYPE_TO_JSON.id()
        ));
    }

    #[test]
    fn test_sets_new_data_type_to_column() {
        let report = lint("alter table books alter column data type jsonb;").unwrap();
        assert!(matched_lint_rule(&report, rules::CHANGE_COLUMN_TYPE.id()));
    }
}
