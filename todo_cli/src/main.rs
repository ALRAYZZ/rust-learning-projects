use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "todo")]
#[command(about = "Simple CLI Todo Application")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

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
