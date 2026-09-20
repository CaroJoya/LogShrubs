// tests/integration.rs

use assert_cmd::Command;
use predicates::prelude::*;

fn loglens() -> Command {
    Command::cargo_bin("loglens").expect("binary builds")
}

fn fixture(name: &str) -> String {
    format!("tests/fixtures/{}", name)
}

// ---------- version ----------

#[test]
fn version_prints_version() {
    loglens()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("loglens 0.1.0"));
}

#[test]
fn version_json_has_name_and_version() {
    loglens()
        .args(["version", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"loglens\""))
        .stdout(predicate::str::contains("\"version\": \"0.1.0\""));
}

// ---------- stats ----------

#[test]
fn stats_counts_levels() {
    let out = loglens()
        .args(["stats", &fixture("simple.log")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();

    assert!(s.contains("INFO"), "INFO section missing: {}", s);
    assert!(s.contains("DEBUG"), "DEBUG section missing: {}", s);
    assert!(s.contains("WARN"), "WARN section missing: {}", s);
    assert!(s.contains("ERROR"), "ERROR section missing: {}", s);
    assert!(s.contains("Lines       4"), "line count wrong: {}", s);
}

// ---------- errors ----------

#[test]
fn errors_groups_repeated_messages() {
    loglens()
        .args(["errors", &fixture("with_errors.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("Database timeout"))
        .stdout(predicate::str::contains("Auth failed for user <NUM>"))
        .stdout(predicate::str::contains("Total       6"));
}

#[test]
fn errors_respects_top_flag() {
    let out = loglens()
        .args(["errors", &fixture("with_errors.log"), "--top", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();

    assert!(s.contains("Database timeout"));
    assert!(
        !s.contains("Auth failed for user"),
        "top=1 still showed second error:\n{}",
        s
    );
}

// ---------- http ----------

#[test]
fn http_counts_status_classes() {
    loglens()
        .args(["http", &fixture("with_http.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("Requests    6"))
        .stdout(predicate::str::contains("/api/products"))
        .stdout(predicate::str::contains("/api/users"));
}

// ---------- analyze ----------

#[test]
fn analyze_produces_full_report() {
    loglens()
        .args(["analyze", &fixture("simple.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("ANALYSIS"))
        .stdout(predicate::str::contains("LEVELS"))
        .stdout(predicate::str::contains("TIME"))
        .stdout(predicate::str::contains(
            "File        tests/fixtures/simple.log",
        ));
}

#[test]
fn analyze_json_is_valid_and_has_expected_keys() {
    let out = loglens()
        .args(["analyze", &fixture("simple.log"), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v: serde_json::Value = serde_json::from_slice(&out).expect("valid JSON");

    assert_eq!(v["total_lines"], 4);
    assert_eq!(v["stats"]["info"], 1);
    assert_eq!(v["stats"]["debug"], 1);
    assert_eq!(v["stats"]["warn"], 1);
    assert_eq!(v["stats"]["error"], 1);
    assert_eq!(v["stats"]["fatal"], 0);
    assert!(v["source"].as_str().unwrap().ends_with("simple.log"));
}

// ---------- error handling ----------

#[test]
fn missing_file_produces_friendly_error() {
    loglens()
        .args(["analyze", "does_not_exist.log"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("file not found"))
        .stderr(predicate::str::contains("does_not_exist.log"));
}

// ---------- robustness ----------

#[test]
fn empty_file_does_not_crash() {
    loglens()
        .args(["analyze", &fixture("empty.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("Lines       0"));
}

#[test]
fn garbage_file_does_not_crash() {
    loglens()
        .args(["analyze", &fixture("garbage.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("ANALYSIS"));
}

// ---------- spike detection ----------

#[test]
fn detects_error_spike() {
    loglens()
        .args(["analyze", &fixture("with_spike.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("ERROR SPIKES"))
        .stdout(predicate::str::contains("15:40"));
}