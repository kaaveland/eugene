use anyhow::{Context, Result};
use chrono::DateTime;
use rayon::prelude::*;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use crate::output::{full_trace_data, GenericHint, Settings};
use crate::{
    generate_new_test_db, hint_data, lints, output, perform_trace, ConnectionSettings,
    TraceSettings,
};

#[test]
fn every_lint_has_an_example_migration() -> Result<()> {
    let example_folder = fs::read_dir("examples")?;
    let mut children = vec![];
    for entry in example_folder {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            children.push(path.file_name().context("No file name")?.to_os_string());
        }
    }
    for hint in hint_data::ALL.iter() {
        let id: OsString = hint.id.into();
        assert!(
            children.contains(&id),
            "No example migration for {}",
            hint.id
        );
        let bad_example_path = format!("examples/{}/bad", hint.id);
        let entry = fs::read_dir(bad_example_path)?;
        assert!(
            entry.count() > 0,
            "No example of bad migration for {}",
            hint.id
        );
    }

    Ok(())
}

fn sorted_dir_files(path: &str) -> Result<Vec<PathBuf>> {
    let dir = fs::read_dir(path)?;
    let mut entries = vec![];
    for entry in dir {
        let path = entry?.path();
        if path.is_file() {
            entries.push(path);
        }
    }
    entries.sort();
    Ok(entries)
}

fn snapshot_lint(id: &str, subfolder: &str) -> Result<String> {
    let example_path = format!("examples/{}/{}", id, subfolder);
    let mut reports = vec![];
    for script in sorted_dir_files(example_path.as_str())? {
        let path = script
            .to_str()
            .context("Path is not a valid UTF-8 string")?;
        let sql = fs::read_to_string(&script)?;
        let report = lints::lint(Some(path.into()), sql, &[])?;
        reports.push(output::markdown::lint_report_to_markdown(&report));
    }
    Ok(reports.join("\n"))
}

fn snapshot_trace(id: &str, subfolder: &str) -> Result<String> {
    let example_path = format!("examples/{}/{}", id, subfolder);
    let mut reports = vec![];
    let db = generate_new_test_db();

    for script in sorted_dir_files(example_path.as_str())? {
        let path = script
            .to_str()
            .context("Path is not a valid UTF-8 string")?;
        let trace_settings = TraceSettings::new(path.into(), true, &[])?;
        let connection_settings = ConnectionSettings::new(
            "postgres".to_string(),
            db.clone(),
            "localhost".to_string(),
            5432,
            "postgres".to_string(),
        );
        let trace = perform_trace(&trace_settings, &connection_settings, &[])?;
        let mut report = full_trace_data(&trace, Settings::new(true, true));

        // Try to make the report deterministic
        report.start_time = DateTime::from_str("2024-05-18T00:00:00Z")?;
        report.all_locks_acquired.iter_mut().for_each(|lock| {
            lock.oid = 1;
        });
        for statement_trace in report.statements.iter_mut() {
            statement_trace.duration_millis = 10;
            statement_trace.new_locks_taken.iter_mut().for_each(|lock| {
                lock.oid = 1;
            });
            statement_trace.locks_at_start.iter_mut().for_each(|lock| {
                lock.oid = 1;
            });
        }

        let md = report.to_markdown()?;
        reports.push(md);
    }
    Ok(reports.join("\n"))
}

fn hint_folder<S: AsRef<str>>(id: S) -> String {
    format!("docs/content/docs/hints/{}", id.as_ref())
}

fn write_lints(id: &str) -> Result<()> {
    let hint_folder = hint_folder(id);
    fs::create_dir_all(hint_folder.as_str())?;
    if is_migration_set_up(id, "bad") {
        let preamble = "---\n
title:  Linted matching transaction
weight: 40
---\n\n";
        let bad = snapshot_lint(id, "bad")?;
        let bad_path = format!("{hint_folder}/unsafe_lint.md");
        fs::write(bad_path, format!("{preamble}\n\n{bad}"))?;
    }
    if is_migration_set_up(id, "good") {
        let good = snapshot_lint(id, "good")?;
        let preamble = "---\n
title:  Linted safer transaction
weight: 50
---\n\n";
        let good_path = format!("{hint_folder}/safer_lint.md");
        fs::write(good_path, format!("{preamble}\n\n{good}"))?;
    }
    Ok(())
}

fn is_migration_set_up(id: &str, subfolder: &str) -> bool {
    let example_path = format!("examples/{}/{}/1.sql", id, subfolder);
    fs::metadata(example_path).is_ok()
}

fn write_traces(id: &str) -> Result<()> {
    hint_data::ALL
        .iter()
        .find(|hint| hint.id == id)
        .context("Hint not found")?;
    let hint_folder = hint_folder(id);
    fs::create_dir_all(hint_folder.as_str())?;
    if is_migration_set_up(id, "bad") {
        let preamble = "---\n
title:  Traced matching transaction
weight: 50
---\n\n"
            .to_string();
        let bad = snapshot_trace(id, "bad")?;
        let bad_path = format!("{hint_folder}/unsafe_trace.md");
        fs::write(bad_path, format!("{preamble}\n{bad}"))?;
    }
    if is_migration_set_up(id, "good") {
        let preamble = "---\n
title:  Traced safer transaction
weight: 60
---\n\n"
            .to_string();
        let good = snapshot_trace(id, "good")?;
        let good_path = format!("{hint_folder}/safer_trace.md");
        fs::write(good_path, format!("{preamble}\n{good}"))?;
    }
    Ok(())
}

#[test]
fn snapshot_lints() -> Result<()> {
    for hint in hint_data::ALL.iter() {
        println!("Writing lints for {}", hint.id);
        write_lints(hint.id)?;
    }
    Ok(())
}

#[test]
fn snapshot_traces() -> Result<()> {
    hint_data::ALL
        .into_par_iter()
        .map(|hint| {
            println!("Writing traces for {}", hint.id);
            write_traces(hint.id)
        })
        .collect::<Result<Vec<()>>>()?;
    Ok(())
}

#[test]
fn generate_lint_pages() -> Result<()> {
    for &hint in hint_data::ALL.iter() {
        let hint: GenericHint = hint.into();
        let name = hint.name;
        let id = hint.id;
        let condition = hint.condition;
        let effect = hint.effect;
        let workaround = hint.workaround;
        let mut supported_by = vec![];
        let has_lint = hint.has_lint;
        if has_lint {
            supported_by.push("`eugene lint`");
        }
        let has_trace = hint.has_trace;
        if has_trace {
            supported_by.push("`eugene trace`");
        }
        let supported_by = supported_by.join(", ");
        let weight: i32 = id.as_str()[1..].parse()?;

        let page = format!(
            "---\ntitle: {id} {name}\nweight: {weight}\n---\n\n# {id} {name}\n\n\
            ## Triggered when\n\n\
            {condition}.\n\n\
            ## Effect\n\n\
            {effect}.\n\n\
            ## Workaround\n\n\
            {workaround}.\n\n\
            ## Support\n\n\
            This hint is supported by {supported_by}.\n\n"
        );
        // create the hint folder if it does not exist
        let hint_folder = hint_folder(id);
        fs::create_dir_all(hint_folder.as_str())?;
        let page_path = format!("{hint_folder}/_index.md");
        fs::write(page_path, page)?;
    }
    Ok(())
}
