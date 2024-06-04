use crate::lint_scripts;
use crate::webapp::error::WebAppError;
use axum::Json;
use eugene::output::LintReport;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct ScriptInput {
    script: String,
}

pub async fn json_lint_handler(
    Json(input): Json<ScriptInput>,
) -> Result<Json<Vec<LintReport>>, WebAppError> {
    let reports = lint_scripts(input.script)?;
    Ok(Json(reports))
}
