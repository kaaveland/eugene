pub use crate::lints::ast::StatementSummary;
use pg_query::protobuf::ConstrType;

/// The `ast` module provides a way to describe a parsed SQL statement in a structured way,
/// using simpler trees than the ones provided by `pg_query`.
pub mod ast;

/// Represents mutable state for linting through a single SQL script.
///
/// This struct is used to keep track of new objects, so the lint rules can check
/// visibility to other transactions, usage of lock timeouts and other properties
/// that require state to be kept between different statements.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct LintContext {
    locktimeout: bool,
    created_objects: Vec<(String, String)>,
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
    /// Update the context with the information from a new statement.
    pub fn update_from(&mut self, summary: &StatementSummary) {
        if let StatementSummary::LockTimeout = summary {
            self.locktimeout = true;
        }
        summary.created_objects().iter().for_each(|(schema, name)| {
            self.created_objects
                .push((schema.to_string(), name.to_string()))
        });
    }
}

/// Emit a warning if a statement takes a lock that is visible to other transactions without a timeout
pub fn emit_locktimeout_warning(ctx: &LintContext, summary: &StatementSummary) -> bool {
    let takes_lock = matches!(
        summary,
        StatementSummary::AlterTable { .. }
            | StatementSummary::CreateIndex {
                concurrently: false,
                ..
            }
    );
    let lock_visible_outside_tx = summary
        .lock_targets()
        .iter()
        .any(|(schema, name)| !ctx.has_created_object(schema, name));
    takes_lock && lock_visible_outside_tx && !ctx.has_locktimeout()
}

pub fn emit_constraint_creates_implicit_index(
    ctx: &LintContext,
    summary: &StatementSummary,
) -> bool {
    if let StatementSummary::AlterTable {
        schema,
        name,
        actions,
    } = summary
    {
        !ctx.has_created_object(schema, name)
            && actions.iter().any(|action| {
                matches!(
                    action,
                    ast::AlterTableAction::AddConstraint {
                        constraint_type: ConstrType::ConstrExclusion,
                        ..
                    }
                ) || matches!(
                    action,
                    ast::AlterTableAction::AddConstraint {
                        constraint_type: ConstrType::ConstrUnique | ConstrType::ConstrPrimary,
                        use_index: false,
                        ..
                    }
                )
            })
    } else {
        false
    }
}

pub fn emit_constraint_does_costly_validation_with_lock(
    ctx: &LintContext,
    summary: &StatementSummary,
) -> bool {
    if let StatementSummary::AlterTable {
        schema,
        name,
        actions,
    } = summary
    {
        !ctx.has_created_object(schema, name)
            && actions.iter().any(|action| {
                matches!(
                    action,
                    ast::AlterTableAction::AddConstraint {
                        constraint_type: ConstrType::ConstrCheck | ConstrType::ConstrForeign,
                        valid: true,
                        ..
                    }
                )
            })
    } else {
        false
    }
}

/// Summarize a SQL script into a list of `StatementSummary` trees.
pub fn summarize<S: AsRef<str>>(sql: S) -> anyhow::Result<Vec<StatementSummary>> {
    let statements = pg_query::split_with_parser(sql.as_ref())?;
    let mut parsed = Vec::new();
    for statement in statements {
        let tree = pg_query::parse(statement)?;
        for raw in tree.protobuf.stmts.iter() {
            if let Some(node) = &raw.stmt {
                if let Some(node_ref) = &node.node {
                    parsed.push(ast::describe(&node_ref.to_ref())?);
                }
            }
        }
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lints::StatementSummary;

    #[test]
    fn test_lock_timeout_visibility_rule() {
        let mut ctx = LintContext::default();
        let create = StatementSummary::CreateIndex {
            concurrently: false,
            schema: "public".to_string(),
            target: "books".to_string(),
            idxname: "books_title_idx".to_string(),
        };
        // Locks table
        assert!(emit_locktimeout_warning(&ctx, &create));
        ctx.update_from(&StatementSummary::CreateTable {
            schema: "public".to_string(),
            name: "books".to_string(),
        });
        // Locks index
        assert!(emit_locktimeout_warning(&ctx, &create));
        ctx.update_from(&create);
        // No locktimeout, but only lock objects visible to this tx
        assert!(!emit_locktimeout_warning(&ctx, &create));
    }
    #[test]
    fn test_lock_timeout_with_locktimeout() {
        let mut ctx = LintContext::default();
        let create = StatementSummary::CreateIndex {
            concurrently: false,
            schema: "public".to_string(),
            target: "books".to_string(),
            idxname: "books_title_idx".to_string(),
        };
        // Locks table
        assert!(emit_locktimeout_warning(&ctx, &create));
        ctx.update_from(&StatementSummary::LockTimeout);
        // Locktimeout, no warning
        assert!(!emit_locktimeout_warning(&ctx, &create));
    }

    #[test]
    fn test_locktimeout_without_taking_lock() {
        let ctx: LintContext = LintContext::default();
        let create_table = StatementSummary::CreateTable {
            schema: "public".to_string(),
            name: "books".to_string(),
        };
        assert!(!emit_locktimeout_warning(&ctx, &create_table));
    }

    #[test]
    fn test_adding_check_constraint() {
        let sql = "ALTER TABLE public.books ADD CONSTRAINT check_price CHECK (price > 0)";
        let summary = summarize(sql).unwrap();
        let ctx = LintContext::default();
        assert!(emit_constraint_does_costly_validation_with_lock(
            &ctx,
            &summary[0]
        ));
        let sql = "ALTER TABLE public.books ADD CONSTRAINT check_price CHECK (price > 0) NOT VALID";
        let summary = summarize(sql).unwrap();
        assert!(!emit_constraint_does_costly_validation_with_lock(
            &ctx,
            &summary[0]
        ));
    }

    #[test]
    fn test_adding_fkey_constraint() {
        let sql = "ALTER TABLE public.books ADD CONSTRAINT fkey_author FOREIGN KEY (author_id) REFERENCES public.authors (id)";
        let summary = summarize(sql).unwrap();
        let ctx = LintContext::default();
        assert!(emit_constraint_does_costly_validation_with_lock(
            &ctx,
            &summary[0]
        ));
        let sql = "ALTER TABLE public.books ADD CONSTRAINT fkey_author FOREIGN KEY (author_id) REFERENCES public.authors (id) NOT VALID";
        let summary = summarize(sql).unwrap();
        assert!(!emit_constraint_does_costly_validation_with_lock(
            &ctx,
            &summary[0]
        ));
    }

    #[test]
    fn test_adding_exclusion_constraint() {
        let sql = "ALTER TABLE public.books ADD CONSTRAINT exclude_title EXCLUDE(title WITH =)";
        let summary = summarize(sql).unwrap();
        let mut ctx = LintContext::default();
        assert!(emit_constraint_creates_implicit_index(&ctx, &summary[0]));
        ctx.update_from(&StatementSummary::CreateTable {
            schema: "public".to_string(),
            name: "books".to_string(),
        });
        assert!(!emit_constraint_creates_implicit_index(&ctx, &summary[0]));
    }

    #[test]
    fn test_adding_unique_constraint() {
        let sql = "ALTER TABLE public.books ADD CONSTRAINT unique_title UNIQUE (title)";
        let summary = summarize(sql).unwrap();
        let mut ctx = LintContext::default();
        assert!(emit_constraint_creates_implicit_index(&ctx, &summary[0]));
        let sql =
            "ALTER TABLE public.books ADD CONSTRAINT unique_title UNIQUE using index title_uq_idx";
        let summary = summarize(sql).unwrap();
        assert!(!emit_constraint_creates_implicit_index(&ctx, &summary[0]));
        ctx.update_from(&StatementSummary::CreateTable {
            schema: "public".to_string(),
            name: "books".to_string(),
        });
        let sql = "ALTER TABLE public.books ADD CONSTRAINT unique_title UNIQUE (title)";
        let summary = summarize(sql).unwrap();
        assert!(!emit_constraint_creates_implicit_index(&ctx, &summary[0]));
    }
}
