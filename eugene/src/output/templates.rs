use handlebars::Handlebars;
use once_cell::sync::Lazy;

use crate::output::{FullTraceData, LintReport};

pub(crate) static HBARS: Lazy<Handlebars> = Lazy::new(|| {
    let mut hbars = Handlebars::new();
    hbars.set_strict_mode(true);
    hbars.register_escape_fn(handlebars::no_escape);
    hbars
        .register_template_string("locks_table_md", include_str!("locks_table.md.hbs"))
        .expect("Failed to register lock_table");
    hbars
        .register_template_string("trace_report_md", include_str!("trace_report.md.hbs"))
        .expect("Failed to register trace_report");
    hbars
        .register_template_string("lint_report_md", include_str!("lint_report.md.hbs"))
        .expect("Failed to register lint_report");
    hbars
});

/// Render a markdown report from a `FullTraceData`
pub fn to_markdown(trace: &FullTraceData) -> crate::Result<String> {
    Ok(HBARS.render("trace_report_md", trace)?)
}

pub fn lint_report_to_markdown(report: &LintReport) -> crate::Result<String> {
    Ok(HBARS.render("lint_report_md", report)?)
}

pub fn trace_text(trace: &FullTraceData) -> crate::Result<String> {
    if trace.passed_all_checks {
        Ok(String::new())
    } else {
        let fname = trace.name.as_deref().unwrap_or("unnamed");
        let mut out = String::new();
        for statement in &trace.statements {
            if !statement.triggered_rules.is_empty() {
                let line = statement.line_number;
                for rule in &statement.triggered_rules {
                    let id = rule.id.as_str();
                    let name = rule.name.as_str();
                    let url = rule.url.as_str();
                    out.push_str(&format!("{fname}:{line} {id} {name} {url}\n"));
                }
            }
        }
        out.pop();
        Ok(out)
    }
}

pub fn lint_text(report: &LintReport) -> crate::Result<String> {
    let mut out = String::new();
    let fname = report.name.as_deref().unwrap_or("unnamed");
    for statement in &report.statements {
        if !statement.triggered_rules.is_empty() {
            let line = statement.line_number;
            for rule in &statement.triggered_rules {
                let id = rule.id.as_str();
                let name = rule.name.as_str();
                let url = rule.url.as_str();
                out.push_str(&format!("{fname}:{line} {id} {name} {url}\n"));
            }
        }
    }
    out.pop();
    Ok(out)
}
