pub mod arg_processing;

use crate::arg_processing::Config;
use chrono::{DateTime, Utc};
use std::fmt::Formatter;
use std::fs::{DirEntry, Metadata, ReadDir};
use std::io::ErrorKind;
use std::ops::Add;
use std::path::Path;
use std::time::UNIX_EPOCH;
use std::{fmt, fs, io};

const FLOPPY: &str = "\u{1F4BE}";
const FOLDER: &str = "\u{1F4C1}";

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

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
    let mut header_row = if extended_attr && width > 80 {
        create_extended_attr_header(width)
    } else {
        vec![String::from("Name:"), String::from("=").repeat(width)]
    };

    let mut string_list_of_files = if extended_attr && width > 80 {
        format_each_ext_attr_entry(&files)?
    } else {
        format_each_entry(files, FLOPPY)?
    };
    let mut string_list_of_dirs = format_each_entry(directories, FOLDER)?;
    header_row.append(&mut string_list_of_files);
    header_row.append(&mut string_list_of_dirs);
    Ok(header_row.join("\n"))
}

fn split_into_files_and_dirs(file_collection: ReadDir) -> (Vec<DirEntry>, Vec<DirEntry>) {
    file_collection
        .into_iter()
        .filter_map(|dir_entry| dir_entry.ok())
        .partition(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
}

fn create_extended_attr_header(width: usize) -> Vec<String> {
    let date_created_heading = create_heading_of_width(24usize, "Date Created");
    let date_modified_heading = create_heading_of_width(24usize, "Date Modified");
    let permissions_heading = create_heading_of_width(12usize, "Permissions");
    let remaining_width = width - 60;
    let name_heading = create_heading_of_width(remaining_width, "Name");
    let header = "".to_string();
    vec![
        header
            + name_heading.as_str()
            + date_created_heading.as_str()
            + permissions_heading.as_str()
            + date_modified_heading.as_str(),
        String::from("=").repeat(width),
    ]
}

fn create_heading_of_width(head_width: usize, name: &str) -> String {
    name.to_string()
        .add(" ".repeat(head_width - name.len()).as_str())
}

fn format_each_ext_attr_entry(files: &[DirEntry]) -> Result<Vec<String>, FileEntryParsingError> {
    files.iter().map(format_file_entry_with_ext_attr).collect()
}

fn format_file_entry_with_ext_attr(dir: &DirEntry) -> Result<String, FileEntryParsingError> {
    let file_name_as_path = dir.path();
    let file_name = match file_name_as_path.to_str() {
        Some(file_name) => file_name,
        None => return Err(FileEntryParsingError::FileNameInvalidUnicode),
    };
    let meta_data = match dir.metadata() {
        Ok(meta) => meta,
        Err(error) => {
            return Err(FileEntryParsingError::MissingMetaDataError {
                original_error: error.kind(),
            })
        }
    };
    let date_created = calc_date_created(&meta_data);
    let permissions = if meta_data.permissions().readonly() {
        "read only "
    } else {
        "writable "
    };
    Ok([FLOPPY, file_name, &date_created, permissions].join(" "))
}

fn calc_date_created(meta_data: &Metadata) -> String {
    let created_since_epoch = meta_data
        .created()
        .expect("Not anticipated to run on systems that do not implement date created for files")
        .duration_since(UNIX_EPOCH)
        .expect("Clock may have gone backwards");
    DateTime::<Utc>::from_timestamp(
        created_since_epoch.as_secs() as i64,
        created_since_epoch.subsec_nanos(),
    )
    .expect(
        "An invalid timestamp was provided, given this is from the system this should not happen",
    )
    .format(DATE_FORMAT)
    .to_string()
}

fn format_each_entry(
    dir_entries: Vec<DirEntry>,
    icon: &str,
) -> Result<Vec<String>, FileEntryParsingError> {
    Ok(dir_entries
        .into_iter()
        .filter_map(|entry| convert_dir_entry_to_str(entry).ok())
        .map(|file_name| icon.to_owned() + " " + &file_name)
        .collect())
}

fn convert_dir_entry_to_str(dir_entry: DirEntry) -> Result<String, FileEntryParsingError> {
    let file_name = dir_entry.file_name();
    let normal_str = match file_name.to_str() {
        Some(name) => name,
        None => return Err(FileEntryParsingError::FileNameInvalidUnicode),
    };
    Ok(String::from(normal_str))
}

pub fn manage_output(config: Config) -> std::io::Result<()> {
    let contents = list_contents(&config, 100)?;
    if config.to_file {
        return fs::write(Path::new(config.target_file.as_str()), contents);
    }
    println!("{}", contents);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use std::fs;
    use std::fs::File;
    use std::time::SystemTime;
    use tempfile::*;

    const FILE_1_NAME: &str = "file_1.txt";
    const FILE_2_NAME: &str = "file_2.txt";
    const FLOPPY_ICON: &str = "\u{1F4BE}";
    const FOLDER_ICON: &str = "\u{1F4C1}";

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
    fn includes_folder_icon_for_sub_folders() {
        let (temp_dir, ..) = setup_basic_test();
        let folder_2 = temp_dir.path().join("sub_folder");
        fs::create_dir(folder_2.as_path()).unwrap();
        assert!(folder_2.exists());
        let (config, _temp_dir) = get_typical_config(Some(temp_dir));
        let list_of_contents = list_contents(&config, 100);
        assert_eq!(
            list_of_contents
                .unwrap()
                .lines()
                .filter(|line| line.starts_with(FOLDER_ICON))
                .count(),
            1
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
        };
        let contents = list_contents(&config, 100);
        assert!(contents.is_err());
    }

    #[test]
    fn output_contains_header_row() {
        let (config, _temp_dir) = get_typical_config(None);
        let contents = list_contents(&config, 100).unwrap();
        assert!(contents.starts_with("Name"));
        assert!(!contents.contains("Date Created"));
        assert!(!contents.contains("Date Modified"));
        assert!(!contents.contains("Permissions"));
    }

    #[test]
    fn contains_seperator_row() {
        let (config, _temp_dir) = get_typical_config(None);
        let contents = list_contents(&config, 100).unwrap();
        let expected_row = "=".repeat(100);
        assert!(contents.contains(&expected_row));
    }

    #[test]
    fn contains_a_header_for_extra_attributes_when_configured() {
        let (temp_dir, ..) = setup_basic_test();
        let config = Config {
            target: temp_dir.path().to_str().unwrap().to_string(),
            to_file: false,
            target_file: "".to_string(),
            extended_attributes: true,
        };
        let contents = list_contents(&config, 100).unwrap();
        assert!(contents.starts_with("Name"));
        assert!(contents.contains("Date Created"));
        assert!(contents.contains("Date Modified"));
        assert!(contents.contains("Permissions"));
    }

    #[test]
    fn spaces_out_columns() {
        let (temp_dir, ..) = setup_basic_test();
        let config = Config {
            target: temp_dir.path().to_str().unwrap().to_string(),
            to_file: false,
            target_file: "".to_string(),
            extended_attributes: true,
        };
        // Date Created and Date Modified = 24 each, rest Name
        let expected_header = "Name                                    Date Created            Permissions Date Modified           ";
        let contents = list_contents(&config, 100).unwrap();
        let lines_of_content: Vec<&str> = contents.split('\n').collect();
        let header = lines_of_content[0];
        assert_eq!(expected_header, header);
    }

    #[test]
    fn contains_date_created_attr() {
        let (temp_dir, file_1, _file_2) = setup_basic_test();
        let expected_file_1_modified = file_1.metadata().unwrap().created().unwrap();
        let date_as_time_since_epoch = expected_file_1_modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let sec_component = date_as_time_since_epoch.as_secs();
        let nano_component = date_as_time_since_epoch.subsec_nanos();
        let date_struct =
            DateTime::<Utc>::from_timestamp(sec_component as i64, nano_component).unwrap();
        let expected_date = date_struct.format(DATE_FORMAT).to_string();

        let config = Config {
            target: temp_dir.path().to_str().unwrap().to_string(),
            to_file: false,
            target_file: "".to_string(),
            extended_attributes: true,
        };
        let contents = list_contents(&config, 400).unwrap();
        assert!(contents.contains(expected_date.as_str()));
    }

    #[test]
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
        };
        let contents = list_contents(&config, 400).unwrap();
        let lines: Vec<&str> = contents.split('\n').collect();
        assert_eq!(4, lines.len());
        assert!(lines[2].contains("read only"));
        assert!(!lines[2].contains("writable"));
        assert!(lines[3].contains("writable"));
        assert!(!lines[3].contains("read only"));
    }

    //rows do not contain extended attributes when false
    //long file names are shortened for small widths to maintain extended attributes
    //spaces inserted between fields match intended widths of each column
    //all fields end with one space even if overflowed
    // returns an error when the console wdith is too small for extended attributes
}
