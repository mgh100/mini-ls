use std::env;
use mini_ls::runner::list_contents;
use mini_ls::arg_processing::Config;

fn main() {
  let args: Vec<String> = env::args().collect();
  let config = Config::new(args);
  println!("{}", list_contents(config.target.as_str()));
}