use clap::Parser;
use std::error::Error;
use todo::{run, Args};

fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();

  run(args)?;
  Ok(())
}
