use clap::Parser;
use clap::Subcommand;
use console::style;
use dialoguer::Editor;
use dialoguer::MultiSelect;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use rusqlite::{Connection, Result};
use std::error::Error;

#[derive(Clone, Debug)]
struct Todo {
  body: String,
  id: usize,
  incomplete: bool,
}

impl PartialEq for Todo {
  fn eq(&self, other: &Self) -> bool {
    (self.id == other.id) && (self.body == other.body) && (self.incomplete == other.incomplete)
  }
}

/// Simple todo app
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(arg_required_else_help(true))] // TODO: Remove if tui is added
pub struct Args {
  #[command(subcommand)]
  command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
  /// Add one or more todo items
  Add {
    /// The todo to add
    todos: Vec<String>,
  },

  /// Remove one or more todo items
  Rm {},

  /// Edit a todo item
  Edit {},

  /// Toggle the completion state of a todo
  Toggle {},

  /// List todo items
  List {
    /// Show only incomplete items
    #[arg(short, long)]
    incomplete: bool,
  },

  /// Remove all completed items
  Clean {},
}

pub fn run(args: Args) -> Result<(), Box<dyn Error>> {
  // Create connection to db
  let conn = Connection::open("todos.db")?;

  // Setup db system
  create_db(&conn)?;

  // Parse the args
  match &args.command {
    Some(Commands::Add { todos }) => add(todos.to_vec(), &conn)?,
    Some(Commands::Rm {}) => {
      let targets = match multi_find(&conn) {
        Ok(result) => result,
        _ => panic!("Something went wrong with selection!"),
      };
      rm(targets, &conn)?;
    }
    Some(Commands::Toggle {}) => {
      let targets = match multi_find(&conn) {
        Ok(result) => result,
        _ => panic!("Something went wrong with selection!"),
      };
      toggle(targets, &conn)?;
    }
    Some(Commands::Edit {}) => {
      let target = match fuzzy_find(&conn) {
        Ok(result) => result,
        _ => panic!("Something went wrong with selection!"),
      };
      if let Some(new) = Editor::new()
        .edit(&target.body)
        .expect("Editor had issues!")
      {
        edit(target, new, &conn)?;
      } else {
        println!("Empty todo is not acceptable!");
      }
    }
    Some(Commands::List { incomplete: all }) => list(*all, conn)?,
    _ => {}
  }

  Ok(())
}

fn create_db(conn: &Connection) -> Result<(), Box<dyn Error>> {
  conn.execute(
    "CREATE TABLE IF NOT EXISTS todos (
            id          INTEGER PRIMARY KEY,
            body        TEXT NOT NULL,
            incomplete  BOOL
        )",
    (), // empty list of parameters.
  )?;

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

  Ok(todos)
}

fn collect_todos_all(conn: &Connection) -> Result<Vec<Todo>, Box<dyn Error>> {
  collect_todos("SELECT * FROM todos;".to_string(), &conn)
}

fn collect_todos_incomplete(conn: &Connection) -> Result<Vec<Todo>, Box<dyn Error>> {
  collect_todos("SELECT * FROM todos where incomplete;".to_string(), &conn)
}

fn fuzzy_find(conn: &Connection) -> Result<Todo, Box<dyn Error>> {
  let todos = collect_todos_all(&conn).unwrap();
  let todo_strs = todos.iter().map(|s| &s.body).collect::<Vec<&String>>();

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
  let todo_strs = todos.iter().map(|s| &s.body).collect::<Vec<&String>>();

  let target_ids = MultiSelect::with_theme(&ColorfulTheme::default())
    .with_prompt("Which one to edit?")
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

fn add(todos: Vec<String>, conn: &Connection) -> Result<(), Box<dyn Error>> {
  if todos.is_empty() {
    // Untested segment starts, this part needs interactivity
    if let Some(new) = Editor::new().edit("").expect("Editor had issues!") {
      conn.execute(
        "INSERT INTO todos (body, incomplete) VALUES (?1, true)",
        (&new,),
      )?;
      println!("Added: {}", new);
    } else {
      println!("Nothing added!");
    }
    // Untested segment ends
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

fn rm(targets: Vec<Todo>, conn: &Connection) -> Result<(), Box<dyn Error>> {
  for target in targets {
    conn.execute("delete from todos where body is ?1", (&target.body,))?;
    println!("Removed todo: {}", target.body);
  }
  Ok(())
}

fn edit(target: Todo, new: String, conn: &Connection) -> Result<(), Box<dyn Error>> {
  conn.execute(
    "UPDATE todos SET body = ?1 where id is ?2",
    (&new, target.id),
  )?;
  println!("Updated to: {}", new);
  Ok(())
}

fn toggle(targets: Vec<Todo>, conn: &Connection) -> Result<(), Box<dyn Error>> {
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

fn list(incomplete: bool, conn: Connection) -> Result<(), Box<dyn Error>> {
  if let Ok(todos) = if incomplete {
    collect_todos_incomplete(&conn)
  } else {
    collect_todos_all(&conn)
  } {
    for (number, todo) in todos.iter().enumerate() {
      if todo.incomplete {
        println!("{}. {}", number + 1, todo.body,);
      } else {
        let output = format!("{}. {}", number + 1, todo.body);
        println!("{}", style(output).strikethrough());
      }
    }
  } else {
    println!("Something went wrong with collecting!");
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn collect_todos_all_empty() {
    let conn = Connection::open_in_memory().unwrap();
    _ = create_db(&conn);
    let todos = collect_todos_all(&conn).unwrap();
    let compare: Vec<Todo> = vec![];
    assert_eq!(compare, todos);
  }

  #[test]
  fn collect_todos_all_one() {
    // Prepare db connection
    let conn = Connection::open_in_memory().unwrap();
    _ = create_db(&conn);

    // Populate
    _ = conn.execute(
      "INSERT INTO todos (body, incomplete) VALUES (?1, true)",
      ("Milk".to_string(),),
    );

    // Test function
    let todos = collect_todos_all(&conn).unwrap();

    // Assert
    assert_eq!(
      vec![Todo {
        id: 1,
        body: "Milk".to_string(),
        incomplete: true
      }],
      todos
    );
  }
  #[test]
  fn collect_todos_all_multi() {
    let conn = Connection::open_in_memory().unwrap();
    _ = create_db(&conn);
    _ = conn.execute(
      "INSERT INTO todos (body, incomplete) VALUES (?1, true)",
      ("Milk".to_string(),),
    );
    _ = conn.execute(
      "INSERT INTO todos (body, incomplete) VALUES (?1, true)",
      ("Carl".to_string(),),
    );
    let todos = collect_todos_all(&conn).unwrap();
    assert_eq!(
      vec![
        Todo {
          id: 1,
          body: "Milk".to_string(),
          incomplete: true
        },
        Todo {
          id: 2,
          body: "Carl".to_string(),
          incomplete: true
        }
      ],
      todos
    );
  }
  #[test]
  fn add_one() {
    // Prepare db connection
    let conn = Connection::open_in_memory().unwrap();
    _ = create_db(&conn);

    // Test function
    _ = add(vec!["Milk".to_string()], &conn);

    // Collect
    let todos = collect_todos_all(&conn).unwrap();

    // Assert
    assert_eq!(
      vec![Todo {
        id: 1,
        body: "Milk".to_string(),
        incomplete: true
      }],
      todos
    );
  }
  #[test]
  fn add_multi() {
    let conn = Connection::open_in_memory().unwrap();
    _ = create_db(&conn);
    _ = add(vec!["Milk".to_string(), "Carl".to_string()], &conn);

    let todos = collect_todos_all(&conn).unwrap();
    assert_eq!(
      vec![
        Todo {
          id: 1,
          body: "Milk".to_string(),
          incomplete: true
        },
        Todo {
          id: 2,
          body: "Carl".to_string(),
          incomplete: true
        }
      ],
      todos
    );
  }
  #[test]
  fn rm_one() {
    // Prepare db connection
    let conn = Connection::open_in_memory().unwrap();
    _ = create_db(&conn);

    // Populate
    _ = add(vec!["Milk".to_string(), "Carl".to_string()], &conn);

    // Test function
    _ = rm(
      vec![Todo {
        id: 1,
        body: "Milk".to_string(),
        incomplete: true,
      }],
      &conn,
    );
    let todos = collect_todos_all(&conn).unwrap();

    // Assert
    assert_eq!(
      vec![Todo {
        id: 2,
        body: "Carl".to_string(),
        incomplete: true
      }],
      todos
    );
  }
  #[test]
  fn rm_multi() {
    let conn = Connection::open_in_memory().unwrap();
    _ = create_db(&conn);
    _ = add(
      vec!["Milk".to_string(), "Carl".to_string(), "Katia".to_string()],
      &conn,
    );

    _ = rm(
      vec![
        Todo {
          id: 1,
          body: "Milk".to_string(),
          incomplete: true,
        },
        Todo {
          id: 3,
          body: "Katia".to_string(),
          incomplete: true,
        },
      ],
      &conn,
    );
    let todos = collect_todos_all(&conn).unwrap();
    assert_eq!(
      vec![Todo {
        id: 2,
        body: "Carl".to_string(),
        incomplete: true
      }],
      todos
    );
  }
  #[test]
  fn edit_test() {
    // Prepare db connection
    let conn = Connection::open_in_memory().unwrap();
    _ = create_db(&conn);

    // Populate
    _ = add(vec!["Milk".to_string(), "Carl".to_string()], &conn);

    // Test function
    _ = edit(
      Todo {
        id: 1,
        body: "Milk".to_string(),
        incomplete: true,
      },
      "Baptise".to_string(),
      &conn,
    );
    let todos = collect_todos_all(&conn).unwrap();

    // Assert
    assert_eq!(
      vec![
        Todo {
          id: 1,
          body: "Baptise".to_string(),
          incomplete: true,
        },
        Todo {
          id: 2,
          body: "Carl".to_string(),
          incomplete: true
        }
      ],
      todos
    );
  }
  #[test]
  fn toggle_one() {
    // Prepare db connection
    let conn = Connection::open_in_memory().unwrap();
    _ = create_db(&conn);

    // Populate
    _ = add(vec!["Milk".to_string(), "Carl".to_string()], &conn);

    // Test function
    _ = toggle(
      vec![Todo {
        id: 1,
        body: "Milk".to_string(),
        incomplete: true,
      }],
      &conn,
    );
    let todos = collect_todos_all(&conn).unwrap();

    // Assert
    assert_eq!(
      vec![
        Todo {
          id: 1,
          body: "Milk".to_string(),
          incomplete: false,
        },
        Todo {
          id: 2,
          body: "Carl".to_string(),
          incomplete: true
        }
      ],
      todos
    );
  }
  #[test]
  fn toggle_multi() {
    let conn = Connection::open_in_memory().unwrap();
    _ = create_db(&conn);
    _ = add(vec!["Milk".to_string(), "Carl".to_string()], &conn);

    _ = toggle(
      vec![
        Todo {
          id: 1,
          body: "Milk".to_string(),
          incomplete: true,
        },
        Todo {
          id: 2,
          body: "Katia".to_string(),
          incomplete: true,
        },
      ],
      &conn,
    );
    let todos = collect_todos_all(&conn).unwrap();
    assert_eq!(
      vec![
        Todo {
          id: 1,
          body: "Milk".to_string(),
          incomplete: false,
        },
        Todo {
          id: 2,
          body: "Carl".to_string(),
          incomplete: false
        }
      ],
      todos
    );
  }
}
