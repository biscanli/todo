use clap::Parser;
use clap::Subcommand;
use console::style;
use dialoguer::Editor;
use dialoguer::MultiSelect;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use rusqlite::{Connection, Result};
use std::error::Error;

#[derive(Clone)]
struct Todo {
  body: String,
  id: usize,
  incomplete: bool,
}

/// Simple todo app
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
  #[command(subcommand)]
  command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
  /// Add todo
  Add {
    /// The todo to add
    todos: Vec<String>,
  },

  /// Remove todo
  Rm {},

  /// Edit todo
  Edit {},

  /// Check a todo app
  Toggle {},

  /// List todos
  List {
    /// Show only incomplete items
    #[arg(short, long)]
    incomplete: bool,
  },
}

pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
  let conn = Connection::open("todos.db")?;

  // Create table if it doesn't exist
  conn.execute(
    "CREATE TABLE IF NOT EXISTS todos (
            id          INTEGER PRIMARY KEY,
            body        TEXT NOT NULL,
            incomplete  BOOL
        )",
    (), // empty list of parameters.
  )?;

  // Parse the args
  match &args.command {
    Some(Commands::Add { todos }) => add(todos.to_vec(), conn)?,
    Some(Commands::Rm {}) => rm(conn)?,
    Some(Commands::Toggle {}) => toggle(conn)?,
    Some(Commands::Edit {}) => edit(conn)?,
    Some(Commands::List { incomplete: all }) => list(*all, conn)?,
    _ => {}
  }

  Ok(())
}

fn collect_todos(query: String, conn: &Connection) -> Result<Vec<Todo>, Box<dyn Error>> {
  let mut stmt = conn.prepare(&query)?;
  let todos = stmt
    .query_map([], |row| {
      Ok(Todo {
        id: row.get(0)?,
        body: row.get(1)?,
        incomplete: row.get(2)?,
      })
    })?
    .into_iter()
    .filter(|s| s.is_ok())
    .map(|s| s.unwrap())
    .collect::<Vec<Todo>>();

  Ok(todos.clone())
}

fn collect_todos_all(conn: &Connection) -> Result<Vec<Todo>, Box<dyn Error>> {
  collect_todos("SELECT * FROM todos;".to_string(), &conn)
}

fn collect_todos_incomplete(conn: &Connection) -> Result<Vec<Todo>, Box<dyn Error>> {
  collect_todos("SELECT * FROM todos where incomplete;".to_string(), &conn)
}

fn fuzzy_find(conn: &Connection) -> Result<Todo, Box<dyn Error>> {
  let todos = collect_todos_all(&conn).unwrap();
  let todo_strs = todos
    .iter()
    .map(|s| s.body.clone())
    .collect::<Vec<String>>();

  let target_id = FuzzySelect::with_theme(&ColorfulTheme::default())
    .with_prompt("Which one to erase?")
    .default(0)
    .items(&todo_strs[..])
    .interact()
    .unwrap();

  Ok(todos[target_id].clone())
}

fn multi_find(conn: &Connection) -> Result<Vec<Todo>, Box<dyn Error>> {
  let todos = collect_todos_all(&conn).unwrap();
  let todo_strs = todos
    .iter()
    .map(|s| s.body.clone())
    .collect::<Vec<String>>();

  let target_ids = MultiSelect::with_theme(&ColorfulTheme::default())
    .with_prompt("Which one to erase?")
    .items(&todo_strs[..])
    .interact()
    .unwrap();

  // Direct indexing (unsafe if indices could be out of bounds)
  let todos_selected: Vec<_> = target_ids
    .iter()
    .map(|&i| todos[i].clone()) // Direct access
    .collect();

  Ok(todos_selected.clone())
}

fn add(todos: Vec<String>, conn: Connection) -> Result<(), Box<dyn Error>> {
  if todos.is_empty() {
    let new = Editor::new().edit("").unwrap().unwrap();
    conn.execute(
      "INSERT INTO todos (body, incomplete) VALUES (?1, true)",
      (&new,),
    )?;
    println!("Added: {}", new);
  } else {
    for todo in todos {
      conn.execute(
        "INSERT INTO todos (body, incomplete) VALUES (?1, true)",
        (&todo,),
      )?;
      println!("Added: {}", todo);
    }
  }
  Ok(())
}

fn rm(conn: Connection) -> Result<(), Box<dyn Error>> {
  let targets = multi_find(&conn).unwrap();
  for target in targets {
    conn.execute("delete from todos where body is ?1", (&target.body,))?;
    println!("Removed todo: {}", target.body);
  }
  Ok(())
}

fn toggle(conn: Connection) -> Result<(), Box<dyn Error>> {
  let targets = multi_find(&conn).unwrap();
  for target in targets {
    let flipped = if target.incomplete { false } else { true };
    conn.execute(
      "UPDATE todos SET incomplete = ?1 where id is ?2",
      (flipped, target.id),
    )?;
    println!("Toggled: {}", target.body);
  }
  Ok(())
}

fn edit(conn: Connection) -> Result<(), Box<dyn Error>> {
  let target = fuzzy_find(&conn).unwrap();
  let new = Editor::new().edit(&target.body).unwrap().unwrap();

  conn.execute(
    "UPDATE todos SET body = ?1 where id is ?2",
    (&new, target.id),
  )?;
  println!("Updated to: {}", new);
  Ok(())
}

fn list(incomplete: bool, conn: Connection) -> Result<(), Box<dyn Error>> {
  let todos = if incomplete {
    collect_todos_incomplete(&conn)
  } else {
    collect_todos_all(&conn)
  }
  .unwrap();
  for (number, todo) in todos.iter().enumerate() {
    if todo.incomplete {
      println!("{}. {}", number + 1, todo.body,);
    } else {
      let output = format!("{}. {}", number + 1, todo.body);
      println!("{}", style(output).strikethrough());
    }
  }
  Ok(())
}
