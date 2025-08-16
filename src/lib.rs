use clap::Parser;
use clap::Subcommand;
use rusqlite::{Connection, Result};
use std::error::Error;

struct Todo {
  id: usize,
  body: String,
  incomplete: bool,
}

/// Simple todo app
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
  // /// do not ignore entries starting with "."
  // #[arg(short, long)]
  // pub all: bool,
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
  },

  /// Check a todo app
  Check {
    // The todo number to remove
    id: usize,
  },

  /// List todos
  List {
    /// do not ignore entries starting with "."
    #[arg(short, long)]
    all: bool,
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
    Some(Commands::List { all }) => list(*all, conn)?,
    _ => {}
  }

  Ok(())
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

  let target = &todos[id - 1].as_ref().unwrap().id;
  conn.execute("delete from todos where id is ?1", (target,))?;
  println!("Removed todo: {}", id);
  Ok(())
}

fn list(all: bool, conn: Connection) -> Result<(), Box<dyn Error>> {
  let mut stmt = conn.prepare("SELECT * FROM todos where incomplete;")?;
  let todos = stmt.query_map([], |row| {
    Ok(Todo {
      id: row.get(0)?,
      body: row.get(1)?,
      incomplete: row.get(2)?,
    })
  })?;

  println!("Things you still need to do:");
  for (number, todo) in todos.enumerate() {
    if let Ok(found_todo) = todo {
      if all || found_todo.incomplete {
        println!("{}. {}", number + 1, found_todo.body,);
      }
    }
  }
  Ok(())
}
