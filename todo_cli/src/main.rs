use todo_cli::*;
use clap::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Commands::Add { title, description } => {
            let mut tasks = load_tasks()?;
            // Convert into iterator, map projects(extracts) the id field from each task
            // max returns an option of either the max value of task.ids or None if no tasks exist
            // then we have unwrap_or(0) to return 0 if no tasks exist, and add 1 to get the next id
            let next_id = tasks
                .iter()
                .map(|task| task.id)
                .max().unwrap_or(0) + 1;

            let new_task = Task {
                id: next_id,
                title,
                description,
                completed: false,
            };

            tasks.push(new_task);
            save_tasks(&tasks)?; // Passing reference is more performant and a default in Rust
            println!("Task added successfully with ID: {}", next_id);
            Ok(())
        }
        Commands::List => {
            let tasks = load_tasks()?;
            if tasks.is_empty() {
                println!("No tasks found");
            } else {
                for task in tasks {
                    let status = if task.completed { "[✓]" } else { "[ ]" };
                    println!(
                        "{} ID: {} - Title: {} | Description: {}",
                        status, task.id, task.title, task.description
                    );
                }
            }
            Ok(())
        }
        Commands::Complete { id } => {
            let mut tasks = load_tasks()?;
            // Find method returns an Options, so we can combine it with an if let pattern
            // so we are checking if it returned Some(task), means we found the task with that id
            // then do something with it
            // The find method syntax is like saying: Find a task where task.id equals the id passed
            if let Some(task) = tasks.iter_mut().find(|task| task.id == id) {
                task.completed = true;
                save_tasks(&tasks)?;
                println!("Task {} marked as completed", id);
                Ok(())
            } else {
                Err(format!("Task {} not found", id).into())
            }
        }
        Commands::Remove { id } => {
            let mut tasks = load_tasks()?;

            // APPROACH USING POSITION AND REMOVE
            // Is more performant than retain because we stop searching once we find the task
            // also allows us to give better feedback to user
            // But in reality the IO operations are the bottleneck, so performance difference is negligible
            if let Some(task) = tasks.iter().position(|task| task.id == id) {
                tasks.remove(task);
                save_tasks(&tasks)?;
                println!("Task {} removed successfully", id);
                Ok(())
            } else {
                Err(format!("Task {} not found", id).into())
            }

            // APPROACH USING RETAIN

            // Retain method doesn't return success or failure, it just modifies the vector in place
            // So we need a way to know if we actually removed something
            // We can get and before and after length of the vector
            //let original_len = tasks.len();
            // Keep on the vector only the tasks that do not match the id to remove
            //tasks.retain(|task| task.id != id);

            // If original length is the same as current length, means we didn't remove anything
            //if original_len == tasks.len() {
                //return Err(format!("Task {} not found", id).into());
            //}

            //save_tasks(&tasks)?;
            //println!("Task {} removed successfully", id);
            //Ok(())
        }
    }
}

