use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_waybar_claude");

fn run(cache: &Path, args: &[&str], stdin: &str) -> String {
    let mut child = Command::new(BIN)
        .arg("--cache")
        .arg(cache)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap()
}

fn json(s: &str) -> serde_json::Value {
    serde_json::from_str(s.trim()).unwrap()
}

#[test]
fn record_then_show_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("sub/usage.json");

    assert_eq!(json(&run(&cache, &["show"], ""))["class"][1], "idle");

    let far = 4_000_000_000i64;
    let input = format!(
        r#"{{"rate_limits":{{"five_hour":{{"used_percentage":85,"resets_at":{far}}},"seven_day":{{"used_percentage":12,"resets_at":{far}}}}}}}"#
    );
    run(&cache, &["record"], &input);
    let v = json(&run(&cache, &["show"], ""));
    assert_eq!(v["class"][1], "crit");
    assert_eq!(v["percentage"], 85);
    assert!(v["tooltip"].as_str().unwrap().contains("7d"));
}

#[test]
fn record_ignores_input_without_limits() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("usage.json");
    run(&cache, &["record"], r#"{"model":{}}"#);
    assert!(!cache.exists());
}

#[test]
fn watch_prints_once_and_stops_on_closed_pipe() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("usage.json");
    let mut child = Command::new(BIN)
        .arg("--cache")
        .arg(&cache)
        .args(["--watch", "show"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    std::io::BufRead::read_line(
        &mut std::io::BufReader::new(child.stdout.take().unwrap()),
        &mut line,
    )
    .unwrap();
    assert_eq!(json(&line)["class"][1], "idle");
    child.kill().unwrap();
}
