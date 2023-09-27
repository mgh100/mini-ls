use std::fmt;
use std::fmt::Formatter;
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

#[derive(Debug, Clone)]
pub enum ArgParsingError {
    MissingFileOption,
}

impl fmt::Display for ArgParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "missing file argument for -F flag")
    }
}

impl Config {
    pub fn build(args: Vec<String>) -> Result<Config, ArgParsingError> {
        let flags: Vec<Flag> = parse_flags(&args);
        let (to_file, target_file, option_index) = parse_file_output_args(&flags, &args)?;
        let target = if args.len() == 1
            || option_index.is_some_and(|i| i == args.len() - 1)
            || (option_index.is_none() && to_file && args.len() == 2)
        {
            "./".to_string()
        } else {
            args.last()
                .expect("Already checked there are args")
                .to_string()
        };
        Ok(Config {
            target,
            to_file,
            target_file,
        })
    }
}

fn parse_flags(args: &[String]) -> Vec<Flag> {
    let flag_mapper = |(i, arg): (usize, &String)| {
        let flag_option_index = if arg.len() > 2 { None } else { Some(i + 1) };
        let flag_option_text = if arg.len() > 2 {
            Some(arg[2..].to_string())
        } else {
            None
        };
        Flag {
            text: arg.to_string().replace('-', ""),
            flag_option_index,
            flag_option_text,
        }
    };
    args.iter()
        .enumerate()
        .filter(|(i, arg)| *i != 0 && arg.starts_with('-'))
        .map(flag_mapper)
        .collect()
}

fn parse_file_output_args(
    flags: &[Flag],
    args: &[String],
) -> Result<(bool, String, Option<usize>), ArgParsingError> {
    let f_arg = flags.iter().find(|flag| flag.text.starts_with('F'));
    match f_arg {
        Some(flag) => match &flag.flag_option_text {
            Some(option_text) => Ok((true, option_text.to_string(), flag.flag_option_index)),
            None => find_out_file_via_index(flag, args),
        },
        None => Ok((false, "".to_string(), None)),
    }
}

fn find_out_file_via_index(
    flag: &Flag,
    args: &[String],
) -> Result<(bool, String, Option<usize>), ArgParsingError> {
    match flag.flag_option_index {
        Some(option_index) => Ok((
            true,
            match args.get(option_index) {
                Some(option_text) => {
                    if Path::new(option_text).is_dir() {
                        return Err(ArgParsingError::MissingFileOption);
                    }
                    option_text.to_string()
                }
                None => {
                    return Err(ArgParsingError::MissingFileOption);
                }
            },
            flag.flag_option_index,
        )),
        _ => Ok((false, "".to_string(), None)),
    }
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
            String::from("/dev"),
        ];
        let _config = Config::build(args);
        assert!(_config.is_err());
        let error = _config.err().unwrap();
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
}
