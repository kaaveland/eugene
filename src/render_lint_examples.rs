use anyhow::{Context, Result};
use rayon::prelude::*;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use crate::output::{full_trace_data, Settings};
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
        let report = full_trace_data(&trace, Settings::new(true, true));
        let md = report.to_markdown()?;
        reports.push(md);
    }
    Ok(reports.join("\n"))
}

fn write_lints(id: &str) -> Result<()> {
    if is_migration_set_up(id, "bad") {
        let bad = snapshot_lint(id, "bad")?;
        let bad_path = format!("examples/{}/bad_lint.md", id);
        fs::write(bad_path, bad)?;
    }
    if is_migration_set_up(id, "good") {
        let good = snapshot_lint(id, "good")?;
        let good_path = format!("examples/{}/good_lint.md", id);
        fs::write(good_path, good)?;
    }
    Ok(())
}

fn is_migration_set_up(id: &str, subfolder: &str) -> bool {
    let example_path = format!("examples/{}/{}/1.sql", id, subfolder);
    fs::metadata(example_path).is_ok()
}

fn write_traces(id: &str) -> Result<()> {
    if is_migration_set_up(id, "bad") {
        let bad = snapshot_trace(id, "bad")?;
        let bad_path = format!("examples/{}/bad_trace.md", id);
        fs::write(bad_path, bad)?;
    }
    if is_migration_set_up(id, "good") {
        let good = snapshot_trace(id, "good")?;
        let good_path = format!("examples/{}/good_trace.md", id);
        fs::write(good_path, good)?;
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
