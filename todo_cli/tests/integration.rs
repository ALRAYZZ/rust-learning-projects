use assert_cmd::prelude::*; // Brings in cargo_bin! macro
use predicates::prelude::*;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn test_add_and_list_integration() {
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();

    // Add a task
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.env("TODO_FILE", &temp_path);
    cmd.arg("add").arg("Buy Milk").arg("Get whole milk");
    cmd.assert().success().stdout(predicate::str::contains("Task added successfully with ID: 1"));

    // List tasks
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.env("TODO_FILE", &temp_path);
    cmd.arg("list");
    cmd.assert().success().stdout(predicate::str::contains("[ ] ID: 1 - Title: Buy Milk | Description: Get whole milk"));
}

#[test]
fn test_complete_integration() {
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();

    // Setup: Add a task
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.env("TODO_FILE", &temp_path);
    cmd.arg("add").arg("Task to Complete").arg("Desc");
    cmd.assert().success();

    // Complete it
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.env("TODO_FILE", &temp_path);
    cmd.arg("complete").arg("1");
    cmd.assert().success().stdout(predicate::str::contains("Task 1 marked as completed"));

    // Verify via list
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.env("TODO_FILE", &temp_path);
    cmd.arg("list");
    cmd.assert().success().stdout(predicate::str::contains("[✓] ID: 1"));
}

#[test]
fn test_remove_integration() {
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();

    // Setup: Add a task
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.env("TODO_FILE", &temp_path);
    cmd.arg("add").arg("Task to Remove").arg("Desc");
    cmd.assert().success();

    // Remove it
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.env("TODO_FILE", &temp_path);
    cmd.arg("remove").arg("1");
    cmd.assert().success().stdout(predicate::str::contains("Task 1 removed successfully"));

    // Verify via list
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.env("TODO_FILE", &temp_path);
    cmd.arg("list");
    cmd.assert().success().stdout(predicate::str::contains("No tasks found."));
}

#[test]
fn test_complete_nonexistent_integration() {
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();

    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.env("TODO_FILE", &temp_path);
    cmd.arg("complete").arg("999");
    cmd.assert().failure().stderr(predicate::str::contains("not found"));
}

#[test]
fn test_remove_nonexistent_integration() {
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap().to_string();

    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.env("TODO_FILE", &temp_path);
    cmd.arg("remove").arg("999");
    cmd.assert().failure().stderr(predicate::str::contains("not found"));
}