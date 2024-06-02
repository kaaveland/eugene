#[cfg(not(windows))]
#[test]
fn snapshot_cli_help() {
    let debug_eugene = "../target/debug/eugene";
    let release_eugene = "../target/release/eugene";
    // pick the most recent binary that exists
    let eugene = if std::path::Path::new(debug_eugene).exists() {
        debug_eugene
    } else {
        release_eugene
    };
    let output = std::process::Command::new(eugene)
        .arg("help")
        .output()
        .expect("failed to execute process");
    let help = String::from_utf8(output.stdout).unwrap();
    std::fs::write("docs/src/shell_output/help", help).unwrap();
    let output = std::process::Command::new(eugene)
        .arg("help")
        .arg("lint")
        .output()
        .expect("failed to execute process");
    let help = String::from_utf8(output.stdout).unwrap();
    std::fs::write("docs/src/shell_output/lint", help).unwrap();
    let output = std::process::Command::new(eugene)
        .arg("help")
        .arg("trace")
        .output()
        .expect("failed to execute process");
    let help = String::from_utf8(output.stdout).unwrap();
    std::fs::write("docs/src/shell_output/trace", help).unwrap();
}
