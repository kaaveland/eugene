use std::ffi::OsString;
use std::fs;

use chrono::{DateTime, Utc};
use handlebars::Handlebars;
use itertools::Itertools;
use once_cell::sync::Lazy;
use pretty_assertions::assert_eq;
use rayon::prelude::*;
use serde::Serialize;

use crate::error::{ContextualError, InnerError};
use crate::hint_data::{data_by_id, HintId};
use crate::lints::lint;
use crate::output::{full_trace_data, GenericHint, Settings};
use crate::parse_scripts::break_into_files;
use crate::{generate_new_test_db, hint_data, output, perform_trace, ClientSource, SqlScript};

static DEFAULT_SETTINGS: Lazy<Settings> = Lazy::new(|| Settings::new(true, true));
static HBARS: Lazy<Handlebars> = Lazy::new(|| {
    let mut hbars = Handlebars::new();
    hbars.set_strict_mode(true);
    hbars.register_escape_fn(handlebars::no_escape);
    hbars
        .register_template_string("hint_page", include_str!("hint_page.md.hbs"))
        .expect("Failed to register hint_page");
    hbars
        .register_template_string("summary", include_str!("doc_summary.md.hbs"))
        .expect("Failed to register summary");
    hbars
});

#[test]
fn every_lint_has_an_example_migration() -> crate::Result<()> {
    let example_folder = fs::read_dir("examples")?;
    let mut children = vec![];
    for entry in example_folder {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            children.push(
                path.file_name()
                    .ok_or_else(|| InnerError::NotFound.with_context("No file name"))?
                    .to_os_string(),
            );
        }
    }
    for hint in hint_data::ALL.iter() {
        let id: OsString = hint.id.into();
        assert!(
            children.contains(&id),
            "No example migration for {}",
            hint.id
        );
        let bad_example_path = format!("examples/{}/bad.sql", hint.id);
        let script = fs::read_to_string(bad_example_path)?;
        assert!(!script.is_empty(), "Missing example for {}", hint.id);
        // In case we put the wrong path into the static hint data
        assert_eq!(
            script, hint.bad_example,
            "Mismatched example for {}",
            hint.id
        );
        // Same thing
        if let Some(good) = hint.good_example {
            assert_eq!(
                good,
                fs::read_to_string(format!("examples/{}/good.sql", hint.id))?
            );
        }
    }

    Ok(())
}

fn snapshot_lint(id: &str, kind: &str, script: &str) -> crate::Result<String> {
    let mut reports = vec![];
    for (name, sql) in break_into_files(script)? {
        let report = lint(
            name.map(|n| format!("examples/{id}/{kind}/{n}")),
            sql,
            &[],
            false,
        )?;
        reports.push(output::templates::lint_report_to_markdown(&report)?);
    }
    Ok(reports.join("\n"))
}

fn snapshot_trace(
    id: &str,
    kind: &str,
    output_settings: &Settings,
    script: &str,
) -> crate::Result<String> {
    let mut reports = vec![];
    let db = generate_new_test_db();
    let mut connection_settings = ClientSource::new(
        "postgres".to_string(),
        db.clone(),
        "localhost".to_string(),
        5432,
        "postgres".to_string(),
    );

    for (name, script) in break_into_files(script)? {
        let path = format!("examples/{}/{kind}/{}", id, name.unwrap());
        let sql = script.into();
        let sql_script = SqlScript { name: path, sql };

        let trace = perform_trace(&sql_script, &mut connection_settings, &[], true)?;
        let mut report = full_trace_data(&trace, *output_settings);

        // Try to make the report deterministic
        report.start_time = DateTime::parse_from_rfc3339("2021-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        report.all_locks_acquired.iter_mut().for_each(|lock| {
            lock.oid = 1;
            lock.lock_duration_millis = 10;
        });
        report.total_duration_millis = 10;
        for statement_trace in report.statements.iter_mut() {
            statement_trace.duration_millis = 10;
            statement_trace.new_locks_taken.iter_mut().for_each(|lock| {
                lock.oid = 1;
                lock.lock_duration_millis = 10;
            });
            statement_trace.locks_at_start.iter_mut().for_each(|lock| {
                lock.oid = 1;
                lock.lock_duration_millis = 10;
            });
        }

        let md = report.to_markdown()?;
        reports.push(md);
    }
    Ok(reports.join("\n"))
}

fn hint_folder<S: AsRef<str>>(id: S) -> String {
    format!("docs/src/hints/{}", id.as_ref())
}

fn write_lints(id: &str) -> crate::Result<bool> {
    let mut changed = false;
    let data = data_by_id(id).expect("Hint not found");
    let hint_folder = hint_folder(id);
    fs::create_dir_all(hint_folder.as_str())?;
    let bad = snapshot_lint(id, "bad", data.bad_example)?;
    let bad_path = format!("{hint_folder}/unsafe_lint.md");
    let prior = fs::read_to_string(&bad_path).unwrap_or_default();
    if prior != bad {
        changed = true;
    }
    fs::write(bad_path, bad)?;

    if let Some(script) = data.good_example {
        let good = snapshot_lint(id, "good", script)?;
        let good_path = format!("{hint_folder}/safer_lint.md");
        let prior = fs::read_to_string(&good_path).unwrap_or_default();
        if prior != good {
            changed = true;
        }
        fs::write(good_path, good)?;
    }

    Ok(changed)
}

fn is_migration_set_up(id: &str, kind: &str) -> bool {
    let example_path = format!("examples/{}/{}.sql", id, kind);
    fs::metadata(example_path).is_ok()
}

fn write_traces(id: &str) -> crate::Result<bool> {
    let mut out = false;
    let data = data_by_id(id).expect("Hint not found");
    let hint_folder = hint_folder(id);
    fs::create_dir_all(hint_folder.as_str())?;

    let bad = snapshot_trace(id, "bad", &DEFAULT_SETTINGS, data.bad_example)?;
    let bad_path = format!("{hint_folder}/unsafe_trace.md");
    let prior = fs::read_to_string(&bad_path).unwrap_or_default();
    if prior != bad {
        out = true;
    }
    fs::write(bad_path, bad)?;

    if let Some(script) = data.good_example {
        let good = snapshot_trace(id, "good", &DEFAULT_SETTINGS, script)?;
        let good_path = format!("{hint_folder}/safer_trace.md");
        let prior = fs::read_to_string(&good_path).unwrap_or_default();
        if prior != good {
            out = true;
        }
        fs::write(good_path, good)?;
    }

    Ok(out)
}

#[test]
fn snapshot_lints() -> crate::Result<()> {
    let mut changed_lints = vec![];
    for hint in hint_data::ALL.iter() {
        let changed = write_lints(hint.id)?;
        if changed {
            changed_lints.push(hint.id);
        }
    }
    assert!(
        changed_lints.is_empty(),
        "Changed lint snapshots, check in if intentional: {:?}",
        changed_lints
    );
    Ok(())
}

#[test]
fn snapshot_traces() -> crate::Result<()> {
    let results: Vec<crate::Result<(String, bool)>> = hint_data::ALL
        .into_par_iter()
        .map(|hint| write_traces(hint.id).map(|changed| (hint.id.to_string(), changed)))
        .collect();
    let results: crate::Result<Vec<_>> = results.into_iter().collect();
    let changed_snapshots: Vec<_> = results?
        .into_iter()
        .filter(|(_, changed)| *changed)
        .map(|(id, _)| id)
        .collect();
    assert!(
        changed_snapshots.is_empty(),
        "Changed trace snapshots, check in if intentional: {:?}",
        changed_snapshots
    );
    Ok(())
}

#[test]
fn test_trace_with_extra_locks() {
    let output_settings = Settings::new(false, true);
    let data = data_by_id("E10").unwrap();
    let r = snapshot_trace(data.id(), "bad", &output_settings, data.bad_example).unwrap();
    let path = "examples/snapshots/extra_locks.md";
    let prior = fs::read_to_string(path).unwrap_or_default();
    fs::write(path, &r).unwrap();
    assert_eq!(
        prior, r,
        "Extra locks have changed, check in the result if that's expected"
    );
}

#[test]
fn test_trace_with_summary() {
    let output_settings = Settings::new(true, false);
    let data = data_by_id("E10").unwrap();
    let r = snapshot_trace(data.id(), "bad", &output_settings, data.bad_example).unwrap();
    let path = "examples/snapshots/summary.md";
    let prior = fs::read_to_string(path).unwrap_or_default();
    fs::write(path, &r).unwrap();
    assert_eq!(
        prior, r,
        "Summary has changed, check in the result if that's expected"
    );
}

#[test]
fn test_trace_with_summary_and_extra_locks() {
    let output_settings = Settings::new(true, true);
    let data = data_by_id("E10").unwrap();
    let r = snapshot_trace(data.id(), "bad", &output_settings, data.bad_example).unwrap();
    let path = "examples/snapshots/summary_extra_locks.md";
    let prior = fs::read_to_string(path).unwrap_or_default();
    fs::write(path, &r).unwrap();
    assert_eq!(
        prior, r,
        "Summary and extra locks have changed, check in the result if that's expected"
    );
}

#[derive(Serialize)]
struct HintPage<'a> {
    hint: &'a GenericHint,
    example_script: &'a str,
    fixed_example_script: Option<&'a str>,
    supported_by: &'a str,
}

fn read_script(id: &str, kind: &str) -> crate::Result<String> {
    let example_path = format!("examples/{}/{}.sql", id, kind);
    let mut script = fs::read_to_string(example_path)?;
    if script.ends_with('\n') {
        script.pop();
    }
    Ok(script)
}

#[test]
fn generate_lint_pages() -> crate::Result<()> {
    for &hint in hint_data::ALL.iter() {
        let hint: GenericHint = hint.into();
        let example_script = read_script(hint.id.as_str(), "bad")?;
        let fixed_example_script = read_script(hint.id.as_str(), "good").ok();
        let supported_by = match (hint.has_lint, hint.has_trace) {
            (true, true) => "`eugene lint` and `eugene trace`",
            (true, false) => "`eugene lint`",
            (false, true) => "`eugene trace`",
            (false, false) => "no tools",
        };
        let page = HintPage {
            hint: &hint,
            example_script: example_script.as_str(),
            fixed_example_script: fixed_example_script.as_deref(),
            supported_by,
        };
        let page = HBARS.render("hint_page", &page)?;
        // create the hint folder if it does not exist
        let hint_folder = hint_folder(&hint.id);
        fs::create_dir_all(hint_folder.as_str())?;
        let page_path = format!("{hint_folder}/index.md");
        fs::write(page_path, page)?;
    }
    Ok(())
}

#[derive(Serialize)]
struct RuleReference {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct ExampleReference {
    id: String,
    name: String,
    cmd: &'static str,
    problematic: bool,
}

#[derive(Serialize)]
struct TocStructure {
    rules: Vec<RuleReference>,
    examples: Vec<ExampleReference>,
}

#[test]
fn render_toc_for_docbook() {
    let mut rules = vec![];
    let mut examples = vec![];
    for &hint in hint_data::ALL.iter().sorted_by_key(|hint| {
        let weight: i32 = hint.id[1..].parse().unwrap();
        weight
    }) {
        let hint: GenericHint = hint.into();
        let id = hint.id.as_str();
        let name = hint.name;
        rules.push(RuleReference {
            id: id.to_string(),
            name: name.to_string(),
        });
        for cmd in ["lint", "trace"] {
            examples.push(ExampleReference {
                id: id.to_string(),
                name: name.to_string(),
                cmd,
                problematic: true,
            });
            if is_migration_set_up(id, "good") {
                examples.push(ExampleReference {
                    id: id.to_string(),
                    name: name.to_string(),
                    cmd,
                    problematic: false,
                });
            }
        }
    }
    let toc_structure = TocStructure { rules, examples };
    let toc = HBARS.render("summary", &toc_structure).unwrap();
    let prior = fs::read_to_string("docs/src/SUMMARY.md").unwrap_or_default();
    fs::write("docs/src/SUMMARY.md", &toc).unwrap();
    assert_eq!(
        prior, toc,
        "Table of contents has changed, check in the result if that's expected"
    );
}
