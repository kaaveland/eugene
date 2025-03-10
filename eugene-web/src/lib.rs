use eugene::lints::lint;
use eugene::output::LintReport;
use eugene::parse_scripts;

pub mod webapp;

pub fn lint_scripts<S: AsRef<str>>(input: S) -> anyhow::Result<Vec<LintReport>> {
    let files = parse_scripts::break_into_files(input.as_ref())?;
    files
        .into_iter()
        .map(|(name, sql)| Ok(lint(name.map(|s| s.to_string()), sql, &[], true, &[])?))
        .collect()
}
