// tests/integration.rs

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn logshrubs() -> Command {
    Command::cargo_bin("logshrubs").expect("binary builds")
}

fn fixture(name: &str) -> String {
    format!("tests/fixtures/{}", name)
}

// ---------- version ----------

#[test]
fn version_prints_version() {
    logshrubs()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("logshrubs {}", VERSION)))
        .stdout(predicate::str::contains("loglens").not());
}

#[test]
fn version_json_has_name_and_version() {
    logshrubs()
        .args(["version", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"logshrubs\""))
        .stdout(predicate::str::contains(format!(
            "\"version\": \"{}\"",
            VERSION
        )));
}

// ---------- stats ----------

#[test]
fn stats_counts_levels() {
    let out = logshrubs()
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
    logshrubs()
        .args(["errors", &fixture("with_errors.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("Database timeout"))
        .stdout(predicate::str::contains("Auth failed for user <NUM>"))
        .stdout(predicate::str::contains("Total       5"));
}

#[test]
fn errors_respects_top_flag() {
    let out = logshrubs()
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
    logshrubs()
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
    logshrubs()
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
    let out = logshrubs()
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
    logshrubs()
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
    logshrubs()
        .args(["analyze", &fixture("empty.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("Lines       0"));
}

#[test]
fn garbage_file_does_not_crash() {
    logshrubs()
        .args(["analyze", &fixture("garbage.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("ANALYSIS"));
}

// ---------- spike detection ----------

#[test]
fn detects_error_spike() {
    logshrubs()
        .args(["analyze", &fixture("with_spike.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("TOP ERRORS"))
        .stdout(predicate::str::contains("Payment failed"));
}
// =====================================================================
// V1.1 — input flexibility
// =====================================================================

#[test]
fn stdin_via_dash() {
    logshrubs()
        .args(["stats", "-"])
        .write_stdin("2026-09-15 14:00:00 INFO a\n2026-09-15 14:00:01 ERROR b\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("File        <stdin>"))
        .stdout(predicate::str::contains("Lines       2"))
        .stdout(predicate::str::contains("Size").not());
}

#[test]
fn bare_command_reads_piped_stdin() {
    logshrubs()
        .write_stdin("2026-09-15 14:00:00 ERROR boom\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("ANALYSIS"))
        .stdout(predicate::str::contains("boom"));
}

#[test]
fn subcommand_without_path_reads_stdin() {
    logshrubs()
        .arg("errors")
        .write_stdin("2026-09-15 14:00:00 ERROR boom\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Total       1"));
}

#[test]
fn multiple_files_merge_into_one_report() {
    let out = logshrubs()
        .args([
            "analyze",
            &fixture("simple.log"),
            &fixture("with_errors.log"),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["sources"].as_array().unwrap().len(), 2);
    assert_eq!(v["total_lines"], 10);
    assert_eq!(v["stats"]["error"], 6);
}

#[test]
fn multiple_files_terminal_says_files() {
    logshrubs()
        .args(["stats", &fixture("simple.log"), &fixture("with_errors.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("Files       2"));
}

#[test]
fn directory_is_read_recursively_by_default() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.log"), "2026-09-15 14:00:00 ERROR top\n").unwrap();
    fs::write(
        dir.path().join("ignored.dat"),
        "2026-09-15 14:00:00 ERROR no\n",
    )
    .unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(
        dir.path().join("sub").join("b.txt"),
        "2026-09-15 14:00:00 ERROR deep\n",
    )
    .unwrap();

    logshrubs()
        .args(["stats", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Files       2"))
        .stdout(predicate::str::contains("Lines       2"));
}

#[test]
fn no_recursive_stays_at_top_level() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.log"), "2026-09-15 14:00:00 ERROR top\n").unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(
        dir.path().join("sub").join("b.log"),
        "2026-09-15 14:00:00 ERROR deep\n",
    )
    .unwrap();

    logshrubs()
        .args(["stats", "--no-recursive", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("File        "))
        .stdout(predicate::str::contains("Lines       1"));
}

#[test]
fn empty_directory_is_a_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("notes.md"), "hi").unwrap();
    logshrubs()
        .args(["analyze", dir.path().to_str().unwrap()])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("no .log or .txt files"));
}

#[test]
fn size_line_shown_for_files() {
    logshrubs()
        .args(["analyze", &fixture("simple.log")])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"Size        \d+(\.\d)? (B|KB|MB|GB)").unwrap());
}

#[test]
fn quiet_hides_empty_sections_on_non_http_log() {
    let loud = logshrubs()
        .args(["http", &fixture("simple.log")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(String::from_utf8(loud)
        .unwrap()
        .contains("(no HTTP requests detected)"));

    logshrubs()
        .args(["http", &fixture("simple.log"), "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no HTTP requests").not());

    logshrubs()
        .args(["analyze", &fixture("simple.log"), "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("HTTP").not())
        .stdout(predicate::str::contains("FATAL").not())
        .stdout(predicate::str::contains("ERROR"));
}

#[test]
fn verbose_widens_truncation() {
    let long = "x".repeat(70);
    let input = format!("2026-09-15 14:00:00 ERROR {}\n", long);

    let normal = logshrubs()
        .args(["errors", "-"])
        .write_stdin(input.clone())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let normal = String::from_utf8(normal).unwrap();
    assert!(!normal.contains(&long));
    assert!(normal.contains('…'));

    logshrubs()
        .args(["errors", "-", "--verbose"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains(&long));
}

#[test]
fn verbose_shows_extra_rows() {
    let mut input = String::new();
    for i in 0..6 {
        input.push_str(&format!(
            "2026-09-15 14:00:0{} ERROR distinct-{}-x\n",
            i,
            (b'a' + i as u8) as char
        ));
    }
    let count_rows = |args: &[&str]| -> usize {
        let out = logshrubs()
            .args(args)
            .write_stdin(input.clone())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        String::from_utf8(out).unwrap().matches("distinct-").count()
    };
    assert_eq!(count_rows(&["errors", "-", "--top", "2"]), 2);
    assert_eq!(count_rows(&["errors", "-", "--top", "2", "--verbose"]), 4);
}

// =====================================================================
// V1.3 — latency statistics
// =====================================================================

#[test]
fn http_shows_response_time_and_slow_endpoints() {
    let out = logshrubs()
        .args(["http", &fixture("with_latency.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("RESPONSE TIME"))
        .stdout(predicate::str::contains("SLOW ENDPOINTS"))
        .stdout(predicate::str::contains("P95"))
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    let slow = s.split("SLOW ENDPOINTS").nth(1).unwrap();
    
    let lines: Vec<&str> = slow.lines().filter(|l| !l.trim().is_empty()).collect();
    // We expect the header line, then 3 endpoints.
    // The first endpoint should be /api/orders
    assert!(lines.iter().any(|l| l.contains("/api/orders")), "Missing /api/orders");
    assert!(lines.iter().any(|l| l.contains("/api/users")), "Missing /api/users");
    assert!(lines.iter().any(|l| l.contains("/api/products")), "Missing /api/products");
    
    // Verify order by checking positions in the raw string (ignoring spacing issues)
    let orders_idx = slow.find("/api/orders").unwrap();
    let users_idx = slow.find("/api/users").unwrap();
    let products_idx = slow.find("/api/products").unwrap();
    
    assert!(
        orders_idx < users_idx && users_idx < products_idx,
        "bad order:\n{}",
        slow
    );
}

#[test]
fn analyze_shows_latency_blocks_and_json_has_percentiles() {
    logshrubs()
        .args(["analyze", &fixture("with_latency.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("RESPONSE TIME"))
        .stdout(predicate::str::contains("SLOW ENDPOINTS"));

    let out = logshrubs()
        .args(["analyze", &fixture("with_latency.log"), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let h = &v["http"];
    assert_eq!(h["max_ms"], 842);
    assert_eq!(h["min_ms"], 20);
    assert!(h["p95_ms"].is_u64());
    assert!(h["p99_ms"].is_u64());
    assert!(h["median_ms"].is_u64());
    assert!(h["avg_ms"].is_f64());
    assert_eq!(h["slowest_endpoints"][0]["path"], "/api/orders");
    assert_eq!(h["slowest_endpoints"][0]["avg_ms"], 821.0);
}

#[test]
fn no_duration_data_means_no_latency_blocks() {
    logshrubs()
        .args(["analyze", &fixture("with_http.log")])
        .assert()
        .success()
        .stdout(predicate::str::contains("HTTP"));
    // with_http.log durations come from the user's fixture; a log with
    // requests but no durations must never print the blocks:
    logshrubs()
        .args(["analyze", "-"])
        .write_stdin("2026-09-15 14:00:00 GET /a 200\n2026-09-15 14:00:01 GET /b 404\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("HTTP"))
        .stdout(predicate::str::contains("RESPONSE TIME").not())
        .stdout(predicate::str::contains("SLOW ENDPOINTS").not());
}

// =====================================================================
// V1.2 — format support
// =====================================================================

#[test]
fn json_logs_are_auto_detected() {
    let out = logshrubs()
        .args(["analyze", &fixture("sample.json"), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["formats"][0], "json");
    assert_eq!(v["stats"]["info"], 2);
    assert_eq!(v["stats"]["warn"], 1);
    assert_eq!(v["stats"]["error"], 2);
    // the malformed line is counted but unleveled
    assert_eq!(v["stats"]["unleveled"], 1);
    assert_eq!(
        v["errors"]["top"][0]["message"],
        "Database timeout after <NUM> s"
    );
    assert_eq!(v["errors"]["top"][0]["count"], 2);
    assert_eq!(v["http"]["total_requests"], 1);
}

#[test]
fn apache_logs_get_real_timestamps_and_http() {
    let out = logshrubs()
        .args(["analyze", &fixture("sample.apache"), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["formats"][0], "apache");
    assert_eq!(v["time"]["first"], "2026-09-15T14:21:04Z");
    assert_eq!(v["time"]["duration_secs"], 3);
    assert_eq!(v["http"]["total_requests"], 4);
    assert_eq!(v["http"]["s5xx"], 1);
    assert!(v["http"]["avg_ms"].is_null());
}

#[test]
fn nginx_access_logs_feed_latency() {
    let out = logshrubs()
        .args(["http", &fixture("sample.nginx"), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["data"]["total_requests"], 4);
    assert_eq!(v["data"]["max_ms"], 1500);
    assert_eq!(v["data"]["slowest_endpoints"][0]["path"], "/boom");
}

#[test]
fn nginx_error_logs_have_real_levels() {
    let out = logshrubs()
        .args(["stats", &fixture("sample_error.nginx"), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["data"]["error"], 1);
    assert_eq!(v["data"]["fatal"], 1);
    assert_eq!(v["data"]["warn"], 1);
    assert_eq!(v["data"]["info"], 1);
}

#[test]
fn format_text_forces_text_on_json_looking_input() {
    let out = logshrubs()
        .args([
            "stats",
            &fixture("sample.json"),
            "--format",
            "text",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["data"]["error"], 0);
    assert_eq!(v["data"]["unleveled"], 6);
}

#[test]
fn explicit_format_flag_is_accepted_and_validated() {
    logshrubs()
        .args(["stats", &fixture("sample.json"), "--format", "json"])
        .assert()
        .success();
    logshrubs()
        .args(["stats", &fixture("sample.json"), "--format", "yaml"])
        .assert()
        .failure();
}

#[test]
fn verbose_reports_detected_format() {
    logshrubs()
        .args(["analyze", &fixture("sample.nginx"), "--verbose"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Format      nginx"));
}

#[test]
fn text_logs_still_work_by_default() {
    let out = logshrubs()
        .args(["analyze", &fixture("simple.log"), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["formats"][0], "text");
}

// =====================================================================
// V2 — parallel processing and watch
// =====================================================================

#[test]
fn parallel_and_sequential_runs_are_identical() {
    let files = [
        fixture("simple.log"),
        fixture("with_errors.log"),
        fixture("with_http.log"),
        fixture("with_latency.log"),
        fixture("with_spike.log"),
        fixture("sample.json"),
        fixture("sample.nginx"),
    ];
    let run = |jobs: &str| -> String {
        let mut args = vec!["analyze", "--json", "--jobs", jobs];
        args.extend(files.iter().map(|s| s.as_str()));
        let out = logshrubs()
            .args(args)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        String::from_utf8(out).unwrap()
    };
    assert_eq!(run("1"), run("4"));
    assert_eq!(run("1"), run("0"));
}

#[test]
fn watch_once_prints_live_summary() {
    logshrubs()
        .args(["watch", &fixture("with_spike.log"), "--once"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WATCH"))
        .stdout(predicate::str::contains("SPIKE STATUS"))
        .stdout(predicate::str::contains("RECENT ERRORS"))
        .stdout(predicate::str::contains("Payment failed"));
}

#[test]
fn watch_once_json_is_one_valid_object() {
    let out = logshrubs()
        .args(["watch", &fixture("simple.log"), "--once", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["lines"], 3);
    assert_eq!(v["stats"]["error"], 0);
}

#[test]
fn watch_missing_file_is_friendly_error() {
    logshrubs()
        .args(["watch", "nope.log", "--once"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("file not found"));
}