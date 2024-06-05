use eugene::lints::lint;
use eugene::output::LintReport;

pub mod parse_scripts;
pub mod webapp;

pub fn validate_syntax<S: AsRef<str>>(sql: S) -> Option<pg_query::Error> {
    pg_query::parse(sql.as_ref()).err()
}

pub fn lint_scripts<S: AsRef<str>>(input: S) -> anyhow::Result<Vec<LintReport>> {
    let files = parse_scripts::break_into_files(input.as_ref())?;
    files
        .into_iter()
        .map(|(name, sql)| lint(name.map(|s| s.to_string()), sql, &[], true))
        .collect()
}
