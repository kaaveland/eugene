use axum::response::Html;
use axum::Form;
use serde::{Deserialize, Serialize};

use eugene::output::Hint;

use crate::webapp::error::WebAppError;
use crate::webapp::templates;
use crate::{parse_scripts, webapp};

#[derive(Deserialize)]
pub struct LintHtmlRequest {
    sql: String,
}

#[derive(Serialize)]
pub struct TriggeredRule {
    file_name: String,
    line_number: usize,
    rule: Hint,
}

#[derive(Serialize)]
pub struct LintHtmlContext {
    passed: bool,
    triggered_rules: Vec<TriggeredRule>,
}

pub(crate) async fn lint_html(
    Form(form): Form<LintHtmlRequest>,
) -> Result<Html<String>, WebAppError> {
    let scripts = parse_scripts::break_into_files(&form.sql)?;
    let mut context = LintHtmlContext {
        passed: true,
        triggered_rules: vec![],
    };
    for (name, sql) in scripts {
        let report = eugene::lints::lint(name.map(|s| s.to_string()), sql, &[], true)?;
        context.passed = context.passed && report.passed_all_checks;
        for st in report.statements {
            for hint in st.triggered_rules {
                context.triggered_rules.push(TriggeredRule {
                    file_name: name.unwrap_or("unnamed.sql").to_string(),
                    line_number: st.line_number,
                    rule: hint,
                });
            }
        }
    }
    templates::handlebars()
        .render("lint", &context)
        .map(Html)
        .map_err(webapp::error::WebAppError::from)
}
