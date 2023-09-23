use std::{env, process};
use std::error::Error;
use mini_ls::{manage_output};
use mini_ls::arg_processing::Config;

fn main() -> Result<(), Box<dyn Error>> {
  let args: Vec<String> = env::args().collect();
  println!("{}", args.get(0).expect("can't be missing"));
  let config = match Config::build(args) {
    Ok(config) => config,
    Err(error) => {
      println!("Arguments are incorrect due to: {}", error);
      process::exit(1);
    }
  };
  let result = manage_output(config);
  match result {
    Ok(()) => Ok(()),
    Err(error) => {
      println!("Unable to read directory due to: {}", error);
      process::exit(1);
    }
  }
}