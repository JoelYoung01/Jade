use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn jade() -> Command {
    Command::cargo_bin("jade").expect("jade binary")
}

fn temp_db() -> (TempDir, String) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("test.db");
    (dir, path.to_string_lossy().into_owned())
}

#[test]
fn help_root_and_nested_topics() {
    jade()
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains("FEATURES"));

    jade()
        .args(["tasks", "update", "status", "help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ALLOWED VALUES"))
        .stdout(predicate::str::contains("inactive"));

    jade()
        .args(["help", "tasks", "add"])
        .assert()
        .success()
        .stdout(predicate::str::contains("jade tasks add"));
}

#[test]
fn crud_round_trip() {
    let (_dir, db) = temp_db();

    let add = jade()
        .args([
            "--db",
            &db,
            "--json",
            "tasks",
            "add",
            "Buy milk",
            "--due",
            "2026-07-22T09:00:00Z",
            "--tag",
            "errands",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Buy milk"))
        .stdout(predicate::str::contains("inactive"))
        .get_output()
        .stdout
        .clone();

    let add_json: serde_json::Value = serde_json::from_slice(&add).expect("add json");
    let id = add_json["id"].as_str().expect("id");

    jade()
        .args(["--db", &db, "tasks", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Buy milk"));

    jade()
        .args([
            "--db",
            &db,
            "--json",
            "tasks",
            "update",
            "--id",
            id,
            "--status",
            "active",
            "--title",
            "Buy oat milk",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Buy oat milk"))
        .stdout(predicate::str::contains("active"));

    jade()
        .args(["--db", &db, "tasks", "delete", "--id", id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted"));

    jade()
        .args(["--db", &db, "tasks", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(no tasks)"));
}

#[test]
fn list_format_plain_csv_json() {
    let (_dir, db) = temp_db();

    jade()
        .args([
            "--db",
            &db,
            "tasks",
            "add",
            "Format me",
            "--due",
            "2026-07-22T09:00:00Z",
            "--tag",
            "errands",
            "-d",
            "needs milk",
        ])
        .assert()
        .success();

    jade()
        .args(["--db", &db, "tasks", "list", "--format", "plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Format me"))
        .stdout(predicate::str::contains("STATUS"));

    let csv = jade()
        .args(["--db", &db, "tasks", "list", "--format", "csv"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "id,status,due_at,title,description,repeat_cron,tags",
        ))
        .stdout(predicate::str::contains("Format me"))
        .stdout(predicate::str::contains("needs milk"))
        .stdout(predicate::str::contains("errands"))
        .get_output()
        .stdout
        .clone();
    let csv_text = String::from_utf8(csv).expect("utf8");
    assert_eq!(csv_text.lines().count(), 2);

    let json = jade()
        .args(["--db", &db, "tasks", "list", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let tasks: serde_json::Value = serde_json::from_slice(&json).expect("list json");
    let arr = tasks.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["title"], "Format me");

    // Global --json still works as a shorthand for list JSON output.
    jade()
        .args(["--db", &db, "--json", "tasks", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\": \"Format me\""));
}

#[test]
fn history_shows_create_and_update() {
    let (_dir, db) = temp_db();

    let add = jade()
        .args([
            "--db",
            &db,
            "--json",
            "tasks",
            "add",
            "History me",
            "--due",
            "2026-07-22T09:00:00Z",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let add_json: serde_json::Value = serde_json::from_slice(&add).expect("add json");
    let id = add_json["id"].as_str().expect("id");

    jade()
        .args([
            "--db",
            &db,
            "tasks",
            "update",
            "--id",
            id,
            "--status",
            "active",
        ])
        .assert()
        .success();

    jade()
        .args(["--db", &db, "tasks", "history", "--id", id])
        .assert()
        .success()
        .stdout(predicate::str::contains("updated"))
        .stdout(predicate::str::contains("created"))
        .stdout(predicate::str::contains("status: inactive -> active"));

    let history = jade()
        .args(["--db", &db, "--json", "tasks", "history", "--id", id])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let events: serde_json::Value = serde_json::from_slice(&history).expect("history json");
    let arr = events.as_array().expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["event_type"], "updated");
    assert_eq!(arr[1]["event_type"], "created");
}

#[test]
fn update_requires_field() {
    let (_dir, db) = temp_db();

    let add = jade()
        .args(["--db", &db, "--json", "tasks", "add", "Keep"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let add_json: serde_json::Value = serde_json::from_slice(&add).expect("add json");
    let id = add_json["id"].as_str().expect("id");

    jade()
        .args(["--db", &db, "tasks", "update", "--id", id])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no fields to update"));
}
