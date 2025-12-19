use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

// Constant holding the name of the JSON file to store tasks
pub const TODO_FILE: &str = "todo.json";

// Trait defining the interface for different storage backends
pub trait TodoStorage {
    fn load(&self) -> Result<Vec<Task>, Box<dyn std::error::Error>>;
    fn save(&self, tasks: &Vec<Task>) -> Result<(), Box<dyn std::error::Error>>;
}

// JSON file storage implementation of TodoStorage trait
pub struct JsonFileStorage {
    file_path: String,
}

impl JsonFileStorage {
pub fn new() -> Self {
        let file_path = std::env::var("TODO_FILE").ok().unwrap_or_else(|| TODO_FILE.to_string());
        Self { file_path }
    }
}

// I/O operations for JSON file storage
impl TodoStorage for JsonFileStorage {
    fn load(&self) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let path = Path::new(&self.file_path);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(path)?;
        let metadata = file.metadata()?;

        // Handle empty files
        if metadata.len() == 0 {
            return Ok(Vec::new());
        }

        let reader = BufReader::new(file);
        let tasks: Vec<Task> = serde_json::from_reader(reader)?;
        Ok(tasks)
    }

    fn save(&self, tasks: &Vec<Task>) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(&self.file_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &tasks)?;
        Ok(())
    }
}


pub struct TodoList<S: TodoStorage> {
    storage: S,
    tasks: Vec<Task>,
}

// Represents the in memory list of tasks with methods to manipulate it
// and perform I/O operations
impl<S: TodoStorage> TodoList<S> {
    // Load from file into TodoList
    // Returns a Result with either the TodoList struct or an error
    pub fn load(storage: S) -> Result<Self, Box<dyn std::error::Error>> {
        // Calls load method based on the storage type we passed (JSON file in this case)
        let tasks = storage.load()?;
        Ok(Self { tasks, storage })
    }

    // Internal save
    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.storage.save(&self.tasks)
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







// (NOT IN USE) I/O functions for loading and saving tasks to JSON file
// KEPT FOR STUDY PURPOSES

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


#[cfg(test)]
mod tests {
    use crate::{Task, TodoList, TodoStorage};

    // Mock storage struct for testing purposes
    struct MockStorage {
        initial_tasks: Vec<Task>, // Allows pre-populating tasks for load tests
        save_called: std::cell::RefCell<bool> // Tracks if save was called
    }

    impl MockStorage {
        fn new(initial_tasks: Vec<Task>) -> Self {
            Self {
                initial_tasks,
                save_called: std::cell::RefCell::new(false),
            }
        }

        fn was_save_called(&self) -> bool {
            *self.save_called.borrow()
        }
    }

    impl TodoStorage for MockStorage {
        fn load(&self) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
            Ok(self.initial_tasks.clone())
        }

        fn save(&self, _tasks: &Vec<Task>) -> Result<(), Box<dyn std::error::Error>> {
            *self.save_called.borrow_mut() = true;
            Ok(())
        }
    }

    #[test]
    fn test_find_next_id_empty_list() {
        let tasks: Vec<Task> = vec![];
        assert_eq!(Task::find_next_id(&tasks), 1);
    }

    #[test]
    fn test_find_next_id_with_tasks() {
        let tasks = vec![
            Task::new(1, "A".to_string(), "".to_string()),
            Task::new(3, "B".to_string(), "".to_string()), // gap to text max, not length
        ];
        assert_eq!(Task::find_next_id(&tasks), 4);
    }

    #[test]
    fn test_load_with_initial_tasks() {
        let initial = vec![Task::new(1, "Test".to_string(), "Desc".to_string())];
        let storage = MockStorage::new(initial.clone());
        let todo_list = TodoList::load(storage).unwrap();
        assert_eq!(todo_list.tasks.len(), 1);
        assert_eq!(todo_list.tasks[0].title, "Test");
    }

    #[test]
    fn test_add_task_success() {
        let storage = MockStorage::new(vec![]);
        let mut todo_list = TodoList::load(storage).unwrap();
        let next_id = todo_list.add("New Task".to_string(), "Desc".to_string()).unwrap();
        assert_eq!(next_id, 1);
        assert_eq!(todo_list.tasks.len(), 1);
        assert_eq!(todo_list.tasks[0].title, "New Task");
        assert!(todo_list.storage.was_save_called());
    }

    #[test]
    fn test_complete_existing_task() {
        let initial = vec![Task::new(1, "Test".to_string(), "Desc".to_string())];
        let storage = MockStorage::new(initial);
        let mut todo_list = TodoList::load(storage).unwrap();
        todo_list.complete(1).unwrap();
        assert!(todo_list.tasks[0].completed);
        assert!(todo_list.storage.was_save_called());
    }

    #[test]
    fn test_complete_nonexistent_task() {
        let storage = MockStorage::new(vec![]);
        let mut todo_list = TodoList::load(storage).unwrap();
        let result = todo_list.complete(999);
        assert!(result.is_err());
        assert!(!todo_list.storage.was_save_called());
    }

    #[test]
    fn test_remove_existing_task() {
        let initial = vec![Task::new(1, "Test".to_string(), "Desc".to_string())];
        let storage = MockStorage::new(initial);
        let mut todo_list = TodoList::load(storage).unwrap();
        todo_list.remove(1).unwrap();
        assert_eq!(todo_list.tasks.len(), 0);
        assert!(todo_list.storage.was_save_called());
    }

    #[test]
    fn test_remove_nonexistent_task() {
        let storage = MockStorage::new(vec![]);
        let mut todo_list = TodoList::load(storage).unwrap();
        let result = todo_list.remove(999);
        assert!(result.is_err());
        assert!(!todo_list.storage.was_save_called());
    }
}


