

pub struct Config {
  pub target: String,
  pub to_file: bool,
  pub target_file: String,
}

struct Flag {
  text: String,
  flag_option_index: usize
}

impl Config {
  pub fn new(args: Vec<String>) -> Config {
    let flags: Vec<Flag> = parse_flags(&args);
    let (to_file, target_file) = parse_file_output_args(&flags, &args);
    let target = args.last().expect("No directory path supplied").to_string();
    Config { target, to_file, target_file }
  }
}

fn parse_flags(args: &Vec<String>) -> Vec<Flag> {
  let flag_mapper = |(i, arg): (usize, &String)| Flag { text: arg.to_string().replace("-", ""), flag_option_index: i + 1 };
  args.iter().enumerate()
    .filter(|(i, arg)| *i != 0 && arg.starts_with("-"))
    .map(flag_mapper)
    .collect()
}

fn parse_file_output_args(flags: &Vec<Flag>, args: &Vec<String>) -> (bool, String) {
  let f_arg = flags.iter().find(|flag| flag.text.starts_with("F"));
  match f_arg {
    Some(flag) => match &flag.text {
      text if text.len() > 1 => (true, text[1..].to_string()),
      _text if flag.flag_option_index >= args.len() - 1 => panic!("missing file argument for -F flag"),
      _text => (true, args.get(flag.flag_option_index).expect("No argument supplied for -F arg").to_string()),
    },
    None => (false, "".to_string()),
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
}