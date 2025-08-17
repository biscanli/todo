use clap::Parser;
use clap::Subcommand;
use console::style;
use rusqlite::{Connection, Result};
use std::error::Error;

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
  Rm {
    // The todo number to remove
    id: usize,
  },

  /// Edit todo
  Edit {
    // The todo number to remove
    id: usize,

    // The new string to replace with
    new: String,
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
    Some(Commands::Rm { id }) => rm(*id, conn)?,
    Some(Commands::Toggle { id }) => toggle(*id, conn)?,
    Some(Commands::Edit { id, new }) => edit(*id, new.to_string(), conn)?,
    Some(Commands::List { incomplete: all }) => list(*all, conn)?,
    _ => {}
  }

  Ok(())
}

fn find_target(id: usize, conn: &Connection) -> Result<usize, Box<dyn Error>> {
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

  Ok(todos[id - 1].as_ref().unwrap().id)
}

fn add(todo: String, conn: Connection) -> Result<(), Box<dyn Error>> {
  conn.execute(
    "INSERT INTO todos (body, incomplete) VALUES (?1, true)",
    (&todo,),
  )?;
  println!("Added: {}", todo);
  Ok(())
}

fn rm(id: usize, conn: Connection) -> Result<(), Box<dyn Error>> {
  let target = find_target(id, &conn).unwrap();
  conn.execute("delete from todos where id is ?1", (target,))?;
  println!("Removed todo: {}", id);
  Ok(())
}

fn toggle(id: usize, conn: Connection) -> Result<(), Box<dyn Error>> {
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

  let target = todos[id - 1].as_ref().unwrap();
  let flipped = if target.incomplete { false } else { true };
  conn.execute(
    "UPDATE todos SET incomplete = ?1 where id is ?2",
    (flipped, target.id),
  )?;
  println!("Done: {}", id);
  Ok(())
}

fn edit(id: usize, new: String, conn: Connection) -> Result<(), Box<dyn Error>> {
  let target = find_target(id, &conn).unwrap();
  conn.execute("UPDATE todos SET body = ?1 where id is ?2", (new, target))?;
  println!("Updated: {}", id);
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
