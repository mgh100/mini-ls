pub mod arg_processing;
mod output_formatting;

use crate::arg_processing::Config;
use crate::FileEntryParsingError::UnableToCalculatePathLengths;

use output_formatting::FormattingCommand;
use std::fmt::Formatter;
use std::fs::{DirEntry, ReadDir};
use std::io::ErrorKind;

use std::path::Path;
use std::{fmt, fs, io};

#[derive(Debug, Clone)]
pub enum FileEntryParsingError {
    UnableToReadDir {
        target: String,
        original_error: io::ErrorKind,
    },
    FileNameInvalidUnicode,
    MissingMetaDataError {
        original_error: io::ErrorKind,
    },
    UnableToCalculatePathLengths,
}

enum TimeOptions {
    Created,
    Modified,
}

impl fmt::Display for FileEntryParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            FileEntryParsingError::UnableToReadDir {
                target,
                original_error,
            } => write!(
                f,
                "was unable to read the contents of {} due to {:?}",
                target, original_error
            ),
            FileEntryParsingError::FileNameInvalidUnicode => {
                write!(f, "file entry did not consist of valid unicode")
            }
            FileEntryParsingError::MissingMetaDataError { original_error } => {
                write!(f, "unable to read meta data due to {}", original_error)
            }
            UnableToCalculatePathLengths => {
                write!(f, "unable to calculate the length of any paths")
            }
        }
    }
}

impl From<FileEntryParsingError> for io::Error {
    fn from(value: FileEntryParsingError) -> Self {
        match value {
            FileEntryParsingError::UnableToReadDir { original_error, .. } => {
                std::io::Error::from(original_error)
            }
            FileEntryParsingError::FileNameInvalidUnicode => {
                std::io::Error::from(ErrorKind::InvalidData)
            }
            FileEntryParsingError::MissingMetaDataError { original_error, .. } => {
                std::io::Error::from(original_error)
            }
            UnableToCalculatePathLengths => std::io::Error::from(ErrorKind::InvalidData),
        }
    }
}

fn list_contents(config: &Config, width: usize) -> Result<String, FileEntryParsingError> {
    let dir_read = fs::read_dir(&config.target);
    match dir_read {
        Ok(file_collection) => Ok(convert_read_dir_to_filename_collection(
            file_collection,
            config.extended_attributes,
            width,
        )?),
        Err(original_error) => {
            let error_kind = original_error.kind();
            Err(FileEntryParsingError::UnableToReadDir {
                target: config.target.to_string(),
                original_error: error_kind,
            })
        }
    }
}

fn convert_read_dir_to_filename_collection(
    file_collection: ReadDir,
    extended_attr: bool,
    width: usize,
) -> Result<String, FileEntryParsingError> {
    let (directories, files): (Vec<DirEntry>, Vec<DirEntry>) =
        split_into_files_and_dirs(file_collection);
    output_formatting::generate_textual_display(FormattingCommand::new(
        extended_attr,
        width,
        files,
        directories,
    ))
}

fn split_into_files_and_dirs(file_collection: ReadDir) -> (Vec<DirEntry>, Vec<DirEntry>) {
    file_collection
        .into_iter()
        .filter_map(|dir_entry| dir_entry.ok())
        .partition(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
}

pub fn manage_output(config: Config) -> std::io::Result<()> {
    let width = if !config.to_file {
        term_size::dimensions()
            .expect("unable to obtain console width")
            .0
    } else {
        120
    };
    let contents = list_contents(&config, width)?;
    if config.to_file {
        return fs::write(Path::new(config.target_file.as_str()), contents);
    }
    println!("{}", contents);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output_formatting::DATE_FORMAT;
    use chrono::{DateTime, Utc};
    use std::fs::File;
    use std::io::Write;
    use std::time::SystemTime;
    use std::{fs, thread, time};
    use tempfile::*;

    const FILE_1_NAME: &str = "file_1.txt";
    const FILE_2_NAME: &str = "file_2.txt";
    const FLOPPY_ICON: &str = "\u{1F4BE}";

    fn setup_basic_test() -> (TempDir, File, File) {
        let temp_dir = tempdir().unwrap();
        let file_1 = temp_dir.path().join(FILE_1_NAME);
        let file_2 = temp_dir.path().join(FILE_2_NAME);
        let file_1_as_file = File::create(&file_1).unwrap();
        let file_2_as_file = File::create(&file_2).unwrap();
        assert!(file_1.as_path().exists());
        assert!(file_2.as_path().exists());
        (temp_dir, file_1_as_file, file_2_as_file)
    }

    fn get_typical_config(dir: Option<TempDir>) -> (Config, TempDir) {
        let temp_dir = if let Some(dir_arg) = dir {
            dir_arg
        } else {
            setup_basic_test().0
        };
        let args = vec![
            String::from("./mini-ls"),
            String::from(temp_dir.path().to_str().unwrap()),
        ];
        (Config::build(args).unwrap(), temp_dir)
    }

    #[test]
    fn includes_files_inside_folder_in_output() {
        let (config, _temp_dir) = get_typical_config(None);
        let list_of_contents = list_contents(&config, 100);
        let list_of_contents = list_of_contents.unwrap();
        assert!(list_of_contents.contains(FILE_1_NAME));
        assert!(list_of_contents.contains(FILE_1_NAME));
    }

    #[test]
    fn includes_that_the_entry_is_a_file() {
        let (config, _temp_dir) = get_typical_config(None);
        let list_of_contents = list_contents(&config, 100);
        assert_eq!(
            list_of_contents
                .unwrap()
                .lines()
                .filter(|line| line.starts_with(FLOPPY_ICON))
                .count(),
            2
        );
    }

    #[test]
    fn writes_to_file() {
        let (temp_dir, ..) = setup_basic_test();
        let file_1 = temp_dir.path().join("log.txt");
        let config = Config {
            target: temp_dir.path().to_str().unwrap().to_string(),
            to_file: true,
            target_file: file_1.to_str().unwrap().to_string(),
            extended_attributes: false,
            recurse: false,
        };
        manage_output(config).unwrap();
        assert!(file_1.exists());
        let file_content = fs::read_to_string(file_1.as_path()).unwrap();
        assert!(file_content.contains(FILE_1_NAME));
        assert!(file_content.contains(FILE_2_NAME));
    }

    #[test]
    fn returns_an_error_on_non_existent_directories() {
        let (temp_dir, ..) = setup_basic_test();
        let target = temp_dir.path().join("no_folder");
        let config = Config {
            target: target.as_path().to_str().unwrap().to_string(),
            to_file: false,
            target_file: "".to_string(),
            extended_attributes: false,
            recurse: false,
        };
        let contents = list_contents(&config, 100);
        assert!(contents.is_err());
    }

    #[test]
    fn contains_date_created_attr() {
        let (temp_dir, file_1, _file_2) = setup_basic_test();
        let expected_file_1_created = file_1.metadata().unwrap().created().unwrap();
        assert_output_contains_time(&expected_file_1_created, &temp_dir);
    }

    fn assert_output_contains_time(expected_file_1_created: &SystemTime, temp_dir: &TempDir) {
        let expected_date = calc_expected_date_string(expected_file_1_created);

        let config = Config {
            target: temp_dir.path().to_str().unwrap().to_string(),
            to_file: false,
            target_file: "".to_string(),
            extended_attributes: true,
            recurse: false,
        };
        let contents = list_contents(&config, 400).unwrap();
        assert!(contents.contains(expected_date.as_str()));
    }

    pub(crate) fn calc_expected_date_string(expected_file_1_created: &SystemTime) -> String {
        let date_as_time_since_epoch = expected_file_1_created
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let sec_component = date_as_time_since_epoch.as_secs();
        let nano_component = date_as_time_since_epoch.subsec_nanos();
        let date_struct =
            DateTime::<Utc>::from_timestamp(sec_component as i64, nano_component).unwrap();
        let expected_date = date_struct.format(DATE_FORMAT).to_string();
        expected_date
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn contains_permissions_when_extended_attr() {
        let (temp_dir, file_1, _file_2) = setup_basic_test();
        let mut permissions = file_1.metadata().unwrap().permissions();
        permissions.set_readonly(true);
        file_1.set_permissions(permissions).unwrap();
        assert!(file_1.metadata().unwrap().permissions().readonly());
        let config = Config {
            target: temp_dir.path().to_str().unwrap().to_string(),
            to_file: false,
            target_file: "".to_string(),
            extended_attributes: true,
            recurse: false,
        };
        let contents = list_contents(&config, 400).unwrap();
        let lines: Vec<&str> = contents.split('\n').collect();
        assert_eq!(4, lines.len());
        assert!(lines[2].contains("read only"));
        assert!(!lines[2].contains("writable"));
        assert!(lines[3].contains("writable"));
        assert!(!lines[3].contains("read only"));
    }

    #[test]
    fn contains_date_modified() {
        let (temp_dir, mut file_1, _file_2) = setup_basic_test();
        let pause = time::Duration::from_millis(1000);
        thread::sleep(pause);
        file_1.write_all(b"Some text to modifiy the file").unwrap();
        file_1.flush().unwrap();
        let expected_file_1_created = file_1.metadata().unwrap().modified().unwrap();
        assert_output_contains_time(&expected_file_1_created, &temp_dir);
    }

    #[test]
    #[should_panic(expected = "requires minimum console width of 80")]
    fn returns_err_on_too_narrow_terminals() {
        let long_file_name =
            "very_long_filename_to_check_for_shortening_of_filename_on_small_consoles.txt";
        let temp_dir = tempdir().unwrap();
        let file_1 = temp_dir.path().join(long_file_name);
        let file_2 = temp_dir.path().join(FILE_2_NAME);
        File::create(&file_1).unwrap();
        File::create(&file_2).unwrap();
        assert!(file_1.as_path().exists());
        assert!(file_2.as_path().exists());
        let config = Config {
            target: temp_dir.path().to_str().unwrap().to_string(),
            to_file: false,
            target_file: "".to_string(),
            extended_attributes: true,
            recurse: false,
        };
        let inadequate_length = 60; // less than reserved for extended attrs
        let _contents = list_contents(&config, inadequate_length).unwrap();
    }
}
