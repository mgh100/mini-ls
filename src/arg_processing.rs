

pub struct Config {
  pub target: String
}

impl Config {
  pub fn new(args: Vec<String>) -> Config {
    match args.get(1) {
      Some(target) => Config{target: target.to_string()},
      None => Config{target: String::from("")}
    }
  }
}

pub fn get_target(args: Vec<String>) -> Config {
  Config::new(args)
}

#[cfg(test)]
mod tests {
  use super::get_target;

  #[test]
  fn obtains_the_dir_from_args() {
    let args = vec![String::from("./mini-ls"), String::from("/dev")];
    let config = get_target(args);
    assert_eq!(config.target, "/dev");
  }
}