use clap::Parser;
use clap::Subcommand;
use console::style;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
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
    // TODO: Make it a list
    /// The todo to add
    body: String,
  },

  /// Remove todo
  Rm {},

  /// Edit todo
  Edit {
    // The todo number to remove
    id: usize,
  },

  /// Check a todo app
  Toggle {
    // The todo number to remove
    id: usize,
  },

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
    Some(Commands::Add { body }) => add(body.to_string(), conn)?,
    Some(Commands::Rm {}) => rm(conn)?,
    Some(Commands::Toggle { id }) => toggle(*id, conn)?,
    Some(Commands::Edit { id }) => edit(*id, conn)?,
    Some(Commands::List { incomplete: all }) => list(*all, conn)?,
    _ => {}
  }

  Ok(())
}

fn find_target(id: usize, conn: &Connection) -> Result<Todo, Box<dyn Error>> {
  let mut stmt = conn.prepare("SELECT * FROM todos;")?;
  let todos = stmt
    .query_map([], |row| {
      Ok(Todo {
        id: row.get(0)?,
        body: row.get(1)?,
        incomplete: row.get(2)?,
      })
    })?
    .collect::<Vec<Result<Todo>>>();

  Ok(todos[id - 1].as_ref().unwrap().clone())
}

fn fuzzy_find(conn: &Connection) -> Result<String, Box<dyn Error>> {
  let mut stmt = conn.prepare("SELECT * FROM todos;")?;
  let todos = stmt
    .query_map([], |row| Ok(row.get(1)?))?
    .into_iter()
    .filter(|s| s.is_ok())
    .map(|s| s.unwrap())
    .collect::<Vec<String>>();

  let target_id = FuzzySelect::with_theme(&ColorfulTheme::default())
    .with_prompt("Which one to erase?")
    .default(0)
    .items(&todos[..])
    .interact()
    .unwrap();

  Ok(todos[target_id].clone())
}

fn add(todo: String, conn: Connection) -> Result<(), Box<dyn Error>> {
  conn.execute(
    "INSERT INTO todos (body, incomplete) VALUES (?1, true)",
    (&todo,),
  )?;
  println!("Added: {}", todo);
  Ok(())
}

fn rm(conn: Connection) -> Result<(), Box<dyn Error>> {
  let target = fuzzy_find(&conn).unwrap();
  conn.execute("delete from todos where body is ?1", (&target,))?;
  println!("Removed todo: {}", target);
  Ok(())
}

fn toggle(id: usize, conn: Connection) -> Result<(), Box<dyn Error>> {
  let target = find_target(id, &conn).unwrap();
  let flipped = if target.incomplete { false } else { true };
  conn.execute(
    "UPDATE todos SET incomplete = ?1 where id is ?2",
    (flipped, target.id),
  )?;
  println!("Toggled: {}", target.body);
  Ok(())
}

fn edit(id: usize, conn: Connection) -> Result<(), Box<dyn Error>> {
  let target = find_target(id, &conn).unwrap();
  let prompt = format!("Change from '{}': ", (target.body));
  let new: String = Input::with_theme(&ColorfulTheme::default())
    .with_prompt(prompt)
    .interact_text()
    .unwrap();

  conn.execute(
    "UPDATE todos SET body = ?1 where id is ?2",
    (new, target.id),
  )?;
  println!("Updated: {}", target.body);
  Ok(())
}

fn list(incomplete: bool, conn: Connection) -> Result<(), Box<dyn Error>> {
  let mut stmt = conn.prepare(if incomplete {
    "SELECT * FROM todos where incomplete;"
  } else {
    "SELECT * FROM todos;"
  })?;
  let todos = stmt.query_map([], |row| {
    Ok(Todo {
      id: row.get(0)?,
      body: row.get(1)?,
      incomplete: row.get(2)?,
    })
  })?;

  for (number, todo) in todos.enumerate() {
    if let Ok(found_todo) = todo {
      if found_todo.incomplete {
        println!("{}. {}", number + 1, found_todo.body,);
      } else {
        let output = format!("{}. {}", number + 1, found_todo.body);
        println!("{}", style(output).strikethrough());
      }
    }
  }
  Ok(())
}
