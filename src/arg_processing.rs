use dirs::home_dir;
use std::fmt;
use std::fmt::Formatter;
use std::path::Path;

const F_FLAG: &str = "F";
const L_FLAG: &str = "l";

#[derive(PartialEq, Eq)]
enum AllowedFlags {
    F,
    L,
}

impl AllowedFlags {
    fn trans_to_char(flag: AllowedFlags) -> char {
        match flag {
            AllowedFlags::F => 'F',
            AllowedFlags::L => 'l',
            _ => '0',
        }
    }

    fn requires_option(flag: AllowedFlags) -> bool {
        match flag {
            AllowedFlags::F => true,
            AllowedFlags::L => false,
        }
    }
}
enum Argument {
    Flag {
        switch: AllowedFlags,
        flag_option_index: Option<usize>,
        flag_option_text: Option<String>,
    },
    TargetDir {
        target: String,
    },
    Option {
        original_index: usize,
        text: String,
    },
}

#[derive(Debug, Clone)]
pub enum ArgParsingError {
    MissingFileOption,
    UnexpectedArgument { argument: String },
}

impl fmt::Display for ArgParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ArgParsingError::MissingFileOption => write!(f, "missing file argument for -F flag"),
            ArgParsingError::UnexpectedArgument { argument } => {
                write!(f, "unexpected argument provided of {}", argument)
            }
        }
    }
}

pub struct Config {
    pub target: String,
    pub to_file: bool,
    pub target_file: String,
    pub(crate) extended_attributes: bool,
}

impl Config {
    pub fn build(args: Vec<String>) -> Result<Config, ArgParsingError> {
        let flags: Result<Vec<Argument>, ArgParsingError> = parse_flags(&args);
        let flags = match flags {
            Ok(flags) => flags,
            Err(error) => return Err(error),
        };
        let (to_file, target_file) = parse_file_output_args(&flags, &args)?;
        let extended_attributes = parse_extended_attribute_flag(&flags);
        let target = flags
            .iter()
            .find(|flag| matches!(flag, Argument::TargetDir { .. }));
        let target = match target {
            Some(Argument::TargetDir { target }) => target.to_string(),
            _ => "./".to_string(),
        };
        Ok(Config {
            target,
            to_file,
            target_file,
            extended_attributes,
        })
    }
}

fn parse_flags(args: &[String]) -> Result<Vec<Argument>, ArgParsingError> {
    let allowed_flags = [F_FLAG, L_FLAG];
    let filtered_args: Vec<&String> = args.iter().skip(1).collect();
    let mut visited_args = vec![];
    let separated_args = filtered_args.iter().enumerate().flat_map(|(i, arg)| {
        match arg {
            string if string.starts_with('-') && string.len() < 3 => {
                let flag_char = string.strip_prefix('-').expect("string already checked for -");
                match flag_char {
                    F_FLAG => {
                        let flag_option_index = if i <= filtered_args.len() - 2 {Some(i + 1)} else {None};
                        if let Some(index) = flag_option_index { visited_args.push(index) };
                        Ok(vec![Argument::Flag {switch: AllowedFlags::F, flag_option_index, flag_option_text: None,}])},
                    L_FLAG => Ok(vec![Argument::Flag {switch: AllowedFlags::L, flag_option_index: Some(i), flag_option_text: None}]),
                    argument => Err(ArgParsingError::UnexpectedArgument {argument: argument.to_string()}),
                }
            },
            string if string.starts_with('-') && string.len() >= 3 => {
                let flag_chars: Vec<&str> = string.strip_prefix('-').expect("string - already checked for").split("").collect();
                let valid_flag_chars: Vec<&str> = flag_chars.into_iter().filter(|flag_char| allowed_flags.contains(flag_char)).collect();
                let valid_flag_block_length = valid_flag_chars.len();
                let flag_option_text = if valid_flag_block_length == arg.len() - 1 {None} else {Some(arg[valid_flag_block_length..].to_string())};
                let flag_option_index = match flag_option_text {
                    None if (i + 1) < args.len() => {
                        visited_args.push(i + 1);
                        Some(i + 1)
                    },
                    Some(_) => None,
                    None => None,
                };
                Ok(valid_flag_chars.iter().map(|flag_char| match flag_char {
                    flag if *flag == L_FLAG => Argument::Flag {switch: AllowedFlags::L, flag_option_text: None, flag_option_index: None},
                    flag if *flag == F_FLAG => Argument::Flag {switch: AllowedFlags::F, flag_option_text: flag_option_text.clone(), flag_option_index},
                    _ => panic!("There is a missing match arm for all the arguments in the allowed_flags vector"),
                }).collect())
            }
            string if visited_args.contains(&i) => Ok(vec![Argument::Option {original_index: i, text: string.to_string()}]),
            target => Ok(vec![Argument::TargetDir {target: (*target).to_string()}]),
        }
    }).flatten().collect();
    Ok(separated_args)
}

fn parse_file_output_args(
    flags: &[Argument],
    args: &[String],
) -> Result<(bool, String), ArgParsingError> {
    // typical input [Flag, Flag, Option, TargetDir]
    for (i, arg) in flags.iter().enumerate() {
        if let Argument::Flag {
            switch: AllowedFlags::F,
            flag_option_text,
            flag_option_index,
        } = arg
        {
            let file_output = true;
            let file_path = match flag_option_text {
                Some(text) => text,
                None => match flags.get(i + 1) {
                    Some(Argument::Option {
                        text,
                        original_index,
                    }) => text,
                    _ => {
                        return Err(ArgParsingError::MissingFileOption);
                    }
                },
            };
            let file_path = if file_path.starts_with('~') {
                let home_dir = dirs::home_dir();
                let home_dir = match home_dir {
                    None => {
                        return Err(ArgParsingError::UnexpectedArgument {
                            argument: file_path.to_string(),
                        });
                    }
                    Some(home) => home,
                };
                let home_dir = home_dir.to_str();
                let home_dir = match home_dir {
                    None => {
                        return Err(ArgParsingError::UnexpectedArgument {
                            argument: file_path.to_string(),
                        });
                    }
                    Some(home) => home,
                };
                file_path.replace('~', home_dir)
            } else {
                file_path.to_string()
            };
            let file_path_as_path = Path::new(&file_path);
            return if file_path_as_path.is_dir() {
                Err(ArgParsingError::MissingFileOption)
            } else {
                Ok((file_output, file_path.to_string()))
            };
        }
    }
    Ok((false, "".to_string()))
}

fn parse_extended_attribute_flag(flags: &[Argument]) -> bool {
    flags.iter().any(|flag| match flag {
        Argument::Flag { switch, .. } => *switch == AllowedFlags::L,
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn obtains_the_dir_from_args() {
        let args = vec![String::from("./mini-ls"), String::from("~/dev")];
        let config = Config::build(args).unwrap();
        assert_eq!(config.target, "~/dev");
    }

    #[test]
    fn extracts_f_arg_to_config() {
        let args = vec![
            String::from("./mini-ls"),
            String::from("-F"),
            String::from("log.txt"),
            String::from("~/dev"),
        ];
        let config = Config::build(args).unwrap();
        assert!(config.to_file);
    }

    #[test]
    fn extracts_target_dir_when_f_arg() {
        let args = vec![
            String::from("./mini-ls"),
            String::from("-F"),
            String::from("log.txt"),
            String::from("~/dev"),
        ];
        let config = Config::build(args).unwrap();
        assert_eq!(config.target, "~/dev");
    }

    #[test]
    fn returns_an_error_if_missing_file_for_output_with_f_flag() {
        let args = vec![
            String::from("./mini-ls"),
            String::from("-F"),
            String::from("~/dev"),
        ];
        let config = Config::build(args);
        assert!(config.is_err());
        let error = config.err().unwrap();
        assert_eq!(error.to_string(), "missing file argument for -F flag")
    }

    #[test]
    fn accepts_flags_concatenated_with_options() {
        let args = vec![
            String::from("./mini-ls"),
            String::from("-Flog.txt"),
            String::from("~/dev"),
        ];
        let config = Config::build(args).unwrap();
        assert_eq!(config.target, "~/dev");
        assert!(config.to_file);
        assert_eq!(config.target_file, "log.txt");
    }

    #[test]
    fn target_dir_is_working_dir_if_un_supplied_with_concat_args() {
        let args = vec![String::from("./mini-ls"), String::from("-Flog.txt")];
        let config = Config::build(args).unwrap();
        assert_eq!(config.target, "./");
    }

    #[test]
    fn target_dir_is_working_dir_if_un_supplied_with_args() {
        let args = vec![
            String::from("./mini-ls"),
            String::from("-F"),
            String::from("log.txt"),
        ];
        let config = Config::build(args).unwrap();
        assert_eq!(config.target, "./");
    }

    #[test]
    fn target_dir_is_working_dir_if_un_supplied_with_no_args() {
        let args = vec![String::from("./mini-ls")];
        let config = Config::build(args).unwrap();
        assert_eq!(config.target, "./");
    }

    #[test]
    fn config_includes_extended_arg_if_passed() {
        let args = vec![String::from("./mini-ls"), String::from("-l")];
        let config = Config::build(args).unwrap();
        assert!(config.extended_attributes);
    }

    #[test]
    fn config_includes_l_arg_if_concatenated() {
        let args = vec![
            String::from("./mini-ls"),
            String::from("-lF"),
            String::from("log.txt"),
        ];
        let config = Config::build(args).unwrap();
        assert!(config.extended_attributes);
    }

    #[test]
    fn finds_all_flags() {
        let args = vec![
            String::from("./mini-ls"),
            String::from("-lF"),
            String::from("log.txt"),
        ];
        let config = Config::build(args).unwrap();
        assert!(config.extended_attributes);
        assert!(config.to_file);
        assert_eq!(config.target_file, "log.txt");
    }
}
