use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

// Constant holding file name to save tasks locally
const TODO_FILE: &str = "todo.json";


// Struct CLI holds the command line arguments of type Commands
#[derive(Parser)]
#[command(name = "todo")]
#[command(about = "Simple CLI Todo Application")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// Enum Commands holds the different commands for the CLI that we can use
#[derive(Subcommand)]
enum Commands {
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

// Task struct holding the in memory representation of a task
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    id: u32,
    title: String,
    description: String,
    completed: bool
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Add { title, description } => {
            let new_task = Task {
                id: 0,
                title,
                description,
                completed: false,
            };
            println!("Adding {} task as: {}", new_task.description, new_task.title);
        }
        Commands::List => {
            println!("Listing all tasks");
        }
        Commands::Complete { id } => {
            println!("Marking task {} as completed", id);
        }
        Commands::Remove { id } => {
            println!("Removing task {}", id);
        }
    }
}

// We need to return Box<dyn std::error::Error> because serde_json::from_reader
// can return different error types. Saying to compiler that we can return some kind of error
// This pattern has the trade off that we cannot match on specific error types later on
// to handle or direct the logic accordingly. We just get a generic error.
fn load_tasks() -> Result<Vec<Task>, Box<dyn std::error::Error>> {
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

fn save_tasks(tasks: &Vec<Task>) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(TODO_FILE)?; // If file does not exist, create it
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &tasks)?;
    Ok(())
}
