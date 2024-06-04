use crate::lint_scripts;
use crate::webapp::error::WebAppError;
use axum::extract::RawForm;
use eugene::output;

pub async fn raw_lint_handler(RawForm(body): RawForm) -> Result<String, WebAppError> {
    let bytes = body.to_vec();
    let script = String::from_utf8(bytes)?;
    let reports: Result<Vec<_>, _> = lint_scripts(script)?
        .into_iter()
        .map(|report| output::templates::lint_text(&report))
        .collect();
    let reports: Vec<_> = reports?
        .into_iter()
        .filter(|report| !report.trim().is_empty())
        .collect();
    Ok(reports.join("\n"))
}
