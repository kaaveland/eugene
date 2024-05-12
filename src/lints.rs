pub use crate::lints::ast::StatementSummary;

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
}
