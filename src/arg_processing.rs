use std::path::Path;

pub struct Config {
  pub target: String,
  pub to_file: bool,
  pub target_file: String,
}

struct Flag {
  text: String,
  flag_option_index: Option<usize>,
  flag_option_text: Option<String>,
}

impl Config {
  pub fn new(args: Vec<String>) -> Config {
    let flags: Vec<Flag> = parse_flags(&args);
    let (to_file, target_file, option_index) = parse_file_output_args(&flags, &args);
    let target = if args.len() == 1 || option_index.is_some_and(|i| i == args.len() - 1) || (option_index.is_none() && to_file && args.len() == 2) {
      "./".to_string()
    } else {
      args.last().expect("Already checked there are args").to_string()
    };
    Config { target, to_file, target_file }
  }
}

fn parse_flags(args: &Vec<String>) -> Vec<Flag> {
  let flag_mapper = |(i, arg): (usize, &String)| {
    let flag_option_index = if arg.len() > 2 {None } else {Some(i + 1)};
    let flag_option_text = if arg.len() > 2 { Some(arg[2..].to_string())} else {None};
    Flag { text: arg.to_string().replace("-", ""), flag_option_index, flag_option_text }};
  args.iter().enumerate()
    .filter(|(i, arg)| *i != 0 && arg.starts_with("-"))
    .map(flag_mapper)
    .collect()
}

fn parse_file_output_args(flags: &Vec<Flag>, args: &Vec<String>) -> (bool, String, Option<usize>) {
  let f_arg = flags.iter().find(|flag| flag.text.starts_with("F"));
  match f_arg {
    Some(flag) => match &flag.flag_option_text {
      Some(option_text) => (true, option_text.to_string(), flag.flag_option_index),
      None => find_out_file_via_index(flag, args),
    },
    None => (false, "".to_string(), None),
  }
}

fn find_out_file_via_index(flag: &Flag, args: &Vec<String>) -> (bool, String, Option<usize>){
  match flag.flag_option_index {
    Some(option_index) => {
      (true, match args.get(option_index) {
        Some(option_text) => {
          if Path::new(option_text).is_dir() {
            panic!("missing file argument for -F flag")
          }
          option_text.to_string()},
        None => panic!("missing file argument for -F flag")
      }, flag.flag_option_index)
    },
    _ => (false, "".to_string(), None)
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
    let args = vec![String::from("./mini-ls"), String::from("/dev"), ];
    let config = get_target(args);
    assert_eq!(config.target, "/dev");
  }

  #[test]
  fn extracts_f_arg_to_config() {
    let args = vec![String::from("./mini-ls"), String::from("-F"), String::from("log.txt"), String::from("/dev")];
    let config = get_target(args);
    assert_eq!(config.to_file, true);
  }

  #[test]
  fn extracts_target_dir_when_f_arg() {
    let args = vec![String::from("./mini-ls"), String::from("-F"), String::from("log.txt"), String::from("/dev")];
    let config = get_target(args);
    assert_eq!(config.target, "/dev");
  }

  #[test]
  #[should_panic(expected = "missing file argument for -F flag")]
  fn panics_if_file_arg_missing_from_f_flag() {
    let args = vec![String::from("./mini-ls"), String::from("-F"), String::from("/dev")];
    let _config = get_target(args);
  }

  #[test]
  fn accepts_flags_concatenated_with_options() {
    let args = vec![String::from("./mini-ls"), String::from("-Flog.txt"), String::from("/dev")];
    let config = get_target(args);
    assert_eq!(config.target, "/dev");
    assert!(config.to_file);
    assert_eq!(config.target_file, "log.txt");
  }

  #[test]
  fn target_dir_is_working_dir_if_unsupplied_with_concat_args() {
    let args = vec![String::from("./mini-ls"), String::from("-Flog.txt")];
    let config = get_target(args);
    assert_eq!(config.target, "./");
  }

  #[test]
  fn target_dir_is_working_dir_if_unsupplied_with_args() {
    let args = vec![String::from("./mini-ls"), String::from("-F"), String::from("log.txt")];
    let config = get_target(args);
    assert_eq!(config.target, "./");
  }

  #[test]
  fn target_dir_is_working_dir_if_unsupplied_with_no_args() {
    let args = vec![String::from("./mini-ls")];
    let config = get_target(args);
    assert_eq!(config.target, "./");
  }
}