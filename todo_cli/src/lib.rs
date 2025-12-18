use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;


pub struct TodoList {
    tasks: Vec<Task>,
}

// Represents the in memory list of tasks with methods to manipulate it
// and perform I/O operations
impl TodoList {
    // Load from file into TodoList
    // Returns a Result with either the TodoList struct or an error
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let tasks = load_tasks()?;
        Ok(Self { tasks })
    }

    // Internal save
    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        save_tasks(&self.tasks)
    }

    // Add a task to the in memory vector and save to file
    pub fn add(&mut self, title: String, description: String)
        -> Result<u32, Box<dyn std::error::Error>> {

        // Convert into iterator, map projects(extracts) the id field from each task
        // max returns an option of either the max value of task.ids or None if no tasks exist
        // then we have unwrap_or(0) to return 0 if no tasks exist, and add 1 to get the next id
        let next_id = Task::find_next_id(&self.tasks);
        let new_task = Task::new(next_id, title, description);
        self.tasks.push(new_task);
        self.save()?;
        Ok(next_id)
    }

    // List tasks from memory
    pub fn list(&self) {
        if self.tasks.is_empty() {
            println!("No tasks found.");
        } else {
            for task in &self.tasks {
                let status = if task.completed { "[✓]" } else { "[ ]" };
                println!(
                    "{} ID: {} - Title: {} | Description: {}",
                    status, task.id, task.title, task.description
                );
            }
        }
    }

    // Complete a task by id and save the updated vector to file
    pub fn complete(&mut self, id: u32) -> Result<(), Box<dyn std::error::Error>> {
        Task::mark_task_completed(&mut self.tasks[..], id)?;
        self.save()?;
        Ok(())
    }

    // Remove a task from vector by id and save the updated vector to file
    pub fn remove(&mut self, id: u32) -> Result<(), Box<dyn std::error::Error>> {

        // APPROACH USING POSITION AND REMOVE
        // Is more performant than retain because we stop searching once we find the task
        // also allows us to give better feedback to user
        // But in reality the IO operations are the bottleneck, so performance difference is negligible
        if let Some(pos) = self.tasks.iter().position(|t| t.id == id) {
            self.tasks.remove(pos);
            self.save()?;
            Ok(())
        } else {
            Err(format!("Task {} not found", id).into())
        }

        // APPROACH USING RETAIN (NOT IN USE)

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

impl Task {
   pub fn new(id: u32, title: String, description: String) -> Self {
        Self {
            id,
            title,
            description,
            completed: false,
        }
   }
    // &[Task] is the default to pass collections as references in Rust way better than
    // passing ownership of the vector
    // Use &[T] slice when only need to read data(looping, searching, etc)
    // Use &mut Vec<T> when you need to modify the collection(add, remove, update)
    // Use Vec<T> when you need to take ownership of the collection(move it somewhere else)
    pub fn find_next_id(tasks: &[Task]) -> u32 {
        tasks
            .iter()
            .map(|task| task.id)
            .max()
            .unwrap_or(0) + 1
    }

    // Find method returns an Options, so we can combine it with an if let pattern
    // so we are checking if it returned Some(task), means we found the task with that id
    // then do something with it
    // The find method syntax is like saying: Find a task where task.id equals the id passed
    pub fn mark_task_completed(tasks: &mut [Task], id: u32) -> Result<(), String> {
        tasks
            .iter_mut()
            .find(|task| task.id == id)
            .map(|task| { task.completed = true })
            .ok_or_else(|| { format!("Task with id {} not found", id) })
    }
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


