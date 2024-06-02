use anyhow::Context;
use once_cell::sync::Lazy;

use regex::Regex;

use crate::hint_data::HintId;

/// A filter rule for lints
#[derive(Eq, PartialEq, Debug)]
pub enum LintAction<'a> {
    SkipAll,
    Skip(Vec<&'a str>),
    Continue,
}

static EUGENE_COMMENT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"-- eugene: ([^\n]+)").expect("Failed to compile regex"));
/// Detect `sql` containing a comment with an instruction for eugene
pub fn find_comment_action(sql: &str) -> anyhow::Result<LintAction> {
    if let Some(captures) = EUGENE_COMMENT_REGEX.captures(sql.as_ref()) {
        let cap = captures
            .get(1)
            .map(|m| m.as_str())
            .context("No capture found")?;
        match cap {
            "ignore" => Ok(LintAction::SkipAll),
            ids if ids.starts_with("ignore ") => {
                let rem = &ids["ignore ".len()..];
                Ok(LintAction::Skip(
                    rem.split(',').map(|id| id.trim()).collect(),
                ))
            }
            _ => Err(anyhow::anyhow!("Unknown eugene instruction: {}", cap)),
        }
    } else {
        Ok(LintAction::Continue)
    }
}

pub fn filter_rules<'a, T: HintId + 'static>(
    filter: &'a LintAction<'a>,
    rules: impl Iterator<Item = &'static T> + 'a,
) -> impl Iterator<Item = &'static T> + 'a {
    rules.filter(move |rule| match filter {
        LintAction::SkipAll => false,
        LintAction::Skip(ids) => !ids.contains(&rule.id()),
        LintAction::Continue => true,
    })
}

#[cfg(test)]
mod tests {
    use crate::lints::rules;
    use crate::lints::rules::LOCKTIMEOUT_WARNING;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn filter_rules() {
        let all = super::filter_rules(&LintAction::Continue, rules::all_rules());
        assert_eq!(all.count(), rules::all_rules().count());
        let ban = vec![LOCKTIMEOUT_WARNING.id()];
        let action = LintAction::Skip(ban);
        let mut skip = super::filter_rules(&action, rules::all_rules());
        assert!(!skip.any(|rule| rule.id() == LOCKTIMEOUT_WARNING.id()));
        assert_eq!(
            0,
            super::filter_rules(&LintAction::SkipAll, rules::all_rules()).count()
        );
    }
    #[test]
    fn sql_with_no_comment() {
        let sql = "SELECT * FROM foo;";
        let action = find_comment_action(sql).unwrap();
        assert_eq!(action, super::LintAction::Continue);
    }

    #[test]
    fn sql_with_ignore_all() {
        let sql = "-- eugene: ignore\nselect * from books;";
        let action = find_comment_action(sql).unwrap();
        assert_eq!(action, LintAction::SkipAll);
    }

    #[test]
    fn sql_with_ignore_several() {
        let sql = "-- eugene: ignore 1, 2, 3\nselect * from books;";
        let action = find_comment_action(sql).unwrap();
        assert_eq!(action, LintAction::Skip(vec!["1", "2", "3"]));
    }
}
