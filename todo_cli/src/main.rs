use todo_cli::*;
use clap::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    // Load tasks from file into memory
    let mut todo_list = TodoList::load()?;

    match args.command {
        Commands::Add { title, description } => {
            // Adds task and returns next id
            let next_id = todo_list.add(title, description)?;
            println!("Task added successfully with ID: {}", next_id);
            Ok(())
        }
        Commands::List => {
            todo_list.list();
            Ok(())
        }
        Commands::Complete { id } => {
            todo_list.complete(id)?;
            println!("Task {} marked as completed", id);
            Ok(())
        }
        Commands::Remove { id } => {
            todo_list.remove(id)?;
            println!("Task {} removed successfully", id);
            Ok(())
        }
    }
}

