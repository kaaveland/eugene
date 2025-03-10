use axum::response::Html;
use axum::Form;
use serde::{Deserialize, Serialize};

use crate::webapp;
use eugene::output::{Hint, LintReport};
use eugene::parse_scripts;

use crate::webapp::error::WebAppError;
use crate::webapp::templates;

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
    syntax_errors: Vec<String>,
    exclamation: &'static str,
}

const EXCLAMATIONS: &[&str] = &[
    "Yikes 😱",
    "Uh oh 😳",
    "Oh dear 🫠",
    "Oh deary me 🫣",
    "Terribly sorry, but... 🧐",
];

pub(crate) async fn lint_html(
    Form(form): Form<LintHtmlRequest>,
) -> Result<Html<String>, WebAppError> {
    let scripts = parse_scripts::break_into_files(&form.sql)?;
    let choice = rand::random::<usize>() % EXCLAMATIONS.len();
    let mut context = LintHtmlContext {
        passed: true,
        triggered_rules: vec![],
        syntax_errors: vec![],
        exclamation: EXCLAMATIONS[choice],
    };
    for (name, sql) in scripts {
        let report: eugene::Result<LintReport> =
            eugene::lints::lint(name.map(|s| s.to_string()), sql, &[], true, &[]);
        match report {
            Err(eugene::error::Error {
                inner: eugene::error::InnerError::SqlText(syntax_error),
                ..
            }) => {
                context.passed = false;
                if let Some(name) = name {
                    context
                        .syntax_errors
                        .push(format!("{name}: {:?}", syntax_error));
                } else {
                    context.syntax_errors.push(format!("{:?}", syntax_error));
                }
                Ok(())
            }
            Ok(report) => {
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
                Ok(())
            }
            Err(err) => Err(err),
        }?;
    }
    templates::handlebars()
        .render("lint", &context)
        .map(Html)
        .map_err(webapp::error::WebAppError::from)
}
