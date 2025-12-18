use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;


// Task struct holding the in memory representation of a task
// Instead of making every field pub I could implement a constructor pub fn new
// but, then I would need to implement getters for every field if I wanted to access them outside
// the module. For simplicity, I will just make them public
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub completed: bool
}

// Enum Commands holds the different commands for the CLI that we can use
#[derive(Subcommand)]
pub enum Commands {
    /// Add a new task
    Add {
        /// Title of the task
        title: String,
        /// Description of the task
        description: String,
    },
    /// List all tasks
    List,
    /// Mark a task as completed
    Complete {
        id: u32,
    },
    /// Remove a task
    Remove {
        id: u32,
    }
}

// Struct CLI holds the command line arguments of type Commands
#[derive(Parser)]
#[command(name = "todo")]
#[command(about = "Simple CLI Todo Application")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

pub const TODO_FILE: &str = "todo.json";

// We need to return Box<dyn std::error::Error> because serde_json::from_reader
// can return different error types. Saying to compiler that we can return some kind of error
// This pattern has the trade off that we cannot match on specific error types later on
// to handle or direct the logic accordingly. We just get a generic error.
pub fn load_tasks() -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    let path = Path::new(TODO_FILE);
    // If no file exists, create new vector in memory to start saving tasks
    if !path.exists() {
        return Ok(Vec::new())
    }

    // Try open file, create a BufReader ready to scoop data from drive
    let file = File::open(path)?; // ? operator. If fails on open return the error
    let reader = BufReader::new(file);
    // Start scooping data from file and deserialize into vector of tasks
    let tasks: Vec<Task> = serde_json::from_reader(reader)?;
    Ok(tasks)
}

// Even if we don't do anything else with the task created on main, we still pass a reference
// it's faster than passing ownership and allowing the compiler to optimize memory usage
pub fn save_tasks(tasks: &Vec<Task>) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(TODO_FILE)?; // If file does not exist, create it
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &tasks)?;
    Ok(())
}


