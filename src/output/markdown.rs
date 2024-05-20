use crate::output::{FullTraceData, LintReport};
use handlebars::Handlebars;
use once_cell::sync::Lazy;

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
pub fn to_markdown(trace: &FullTraceData) -> anyhow::Result<String> {
    HBARS
        .render("trace_report_md", trace)
        .map_err(|e| anyhow::anyhow!("Failed to render markdown: {}", e))
}

pub fn lint_report_to_markdown(report: &LintReport) -> anyhow::Result<String> {
    HBARS
        .render("lint_report_md", report)
        .map_err(|e| anyhow::anyhow!("Failed to render markdown: {}", e))
}
