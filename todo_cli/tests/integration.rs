use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_add_and_list_integration() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().join("todo.json");
    let temp_str = temp_path.to_str().unwrap();

    // Add a task
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.arg("add").arg("Buy Milk").arg("Get whole milk");
    cmd.assert().success().stdout(predicate::str::contains("Task added successfully with ID: 1"));

    // List tasks
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.arg("list");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[ ] ID: 1 - Title: Buy Milk | Description: Get whole milk"));

    // Clean up
    fs::remove_file("todo.json").ok();
}

#[test]
fn test_complete_integration() {
    // Setup: Add a task first
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.arg("add").arg("Task to Complete").arg("Desc");
    cmd.assert().success();

    // Complete it
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.arg("complete").arg("1");
    cmd.assert().success().stdout(predicate::str::contains("Task 1 marked as completed"));

    // Verify via list
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.arg("list");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[✓] ID: 1"));

    // Clean up
    fs::remove_file("todo.json").ok();
}

#[test]
fn test_remove_integration() {
    // Setup: Add a task
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.arg("add").arg("Task to Remove").arg("Desc");
    cmd.assert().success();

    // Remove it
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.arg("remove").arg("1");
    cmd.assert().success().stdout(predicate::str::contains("Task 1 removed successfully"));

    // Verify via list
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.arg("list");
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No tasks found."));

    // Clean up (though remove should have emptied it)
    fs::remove_file("todo.json").ok();
}

#[test]
fn test_complete_nonexistent_integration() {
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.arg("complete").arg("999");
    cmd.assert().failure().stderr(predicate::str::contains("not found"));
}

#[test]
fn test_remove_nonexistent_integration() {
    let mut cmd = Command::cargo_bin("todo_cli").unwrap();
    cmd.arg("remove").arg("999");
    cmd.assert().failure().stderr(predicate::str::contains("not found"));
}