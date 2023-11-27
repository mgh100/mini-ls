pub mod arg_processing;

use crate::arg_processing::Config;
use crate::FileEntryParsingError::UnableToCalculatePathLengths;
use crate::TimeOptions::{Created, Modified};
use chrono::{DateTime, Utc};
use std::fmt::Formatter;
use std::fs::{DirEntry, Metadata, ReadDir};
use std::io::ErrorKind;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use std::{fmt, fs, io};
use unicode_segmentation::UnicodeSegmentation;

const FLOPPY: &str = "\u{1F4BE}";
const FOLDER: &str = "\u{1F4C1}";
const RESERVED_LENGTH: usize = 63;

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

struct FormattingCommand {
    extended_attr: bool,
    width: usize,
    files: Vec<DirEntry>,
    longest: usize,
}

impl FormattingCommand {
    fn new(extended_attr: bool, width: usize, files: Vec<DirEntry>, longest: usize) -> Self {
        FormattingCommand {
            extended_attr,
            width,
            files,
            longest,
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
    let Some(longest) = analyse_longest(vec![&directories, &files]) else {
        return Err(UnableToCalculatePathLengths);
    };
    generate_textual_display(
        FormattingCommand::new(extended_attr, width, files, longest),
        directories,
    )
}

fn split_into_files_and_dirs(file_collection: ReadDir) -> (Vec<DirEntry>, Vec<DirEntry>) {
    file_collection
        .into_iter()
        .filter_map(|dir_entry| dir_entry.ok())
        .partition(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
}

fn analyse_longest(entries: Vec<&Vec<DirEntry>>) -> Option<usize> {
    let full_list: Vec<&DirEntry> = entries.iter().flat_map(|vector| vector.iter()).collect();
    full_list
        .into_iter()
        .map(|dir_entry: &DirEntry| dir_entry.path())
        .map(|path: PathBuf| {
            let path_as_str_option = path.to_str();
            let path_as_str = path_as_str_option.unwrap_or("");
            String::from(path_as_str)
        })
        .map(|stringy| stringy.len())
        .max()
}

fn generate_textual_display(
    command: FormattingCommand,
    directories: Vec<DirEntry>,
) -> Result<String, FileEntryParsingError> {
    let mut header_row = if command.extended_attr && command.width > 80 {
        create_extended_attr_header(command.width, command.longest)
    } else {
        vec![
            String::from("Name:"),
            String::from("=").repeat(command.width),
        ]
    };
    let mut string_list_of_files = orchestrate_formatting(command)?;
    let mut string_list_of_dirs = format_each_entry(directories, FOLDER)?;
    header_row.append(&mut string_list_of_files);
    header_row.append(&mut string_list_of_dirs);
    Ok(header_row.join("\n"))
}

fn create_extended_attr_header(width: usize, longest: usize) -> Vec<String> {
    let date_created_heading = create_heading_of_width(24usize, "Date Created");
    let date_modified_heading = create_heading_of_width(24usize, "Date Modified");
    let permissions_heading = create_heading_of_width(12usize, "Permissions");
    let remaining_width = if longest + 3 <= width - 60 {
        longest + 3
    } else {
        width - 60
    };
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

fn orchestrate_formatting(
    command: FormattingCommand,
) -> Result<Vec<String>, FileEntryParsingError> {
    Ok(if command.extended_attr && command.width > 80 {
        let available_filename_space = command.width - 63;
        let file_name_target_length = if available_filename_space > command.longest {
            command.longest
        } else {
            available_filename_space
        };
        format_each_ext_attr_entry(&command.files, file_name_target_length)?
    } else if command.extended_attr && command.width <= 80 {
        panic!("requires minimum console width of 80");
    } else {
        format_each_entry(command.files, FLOPPY)?
    })
}

fn create_heading_of_width(head_width: usize, name: &str) -> String {
    name.to_string()
        .add(" ".repeat(head_width - name.len()).as_str())
}

fn format_each_ext_attr_entry(
    files: &[DirEntry],
    max_file_name_width: usize,
) -> Result<Vec<String>, FileEntryParsingError> {
    files
        .iter()
        .map(|dir| format_file_entry_with_ext_attr(dir, max_file_name_width))
        .collect()
}

fn format_file_entry_with_ext_attr(
    dir: &DirEntry,
    allowed_width: usize,
) -> Result<String, FileEntryParsingError> {
    let file_name_as_path = dir.path();
    let file_name = match file_name_as_path.to_str() {
        Some(file_name) => set_file_name_length(allowed_width, file_name),
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
    let date_created = get_formatted_date(&meta_data, Created);
    let permissions = if meta_data.permissions().readonly() {
        "read only "
    } else {
        "writable "
    };
    let date_modified = get_formatted_date(&meta_data, Modified);
    Ok([
        FLOPPY,
        file_name.as_str(),
        &date_created,
        permissions,
        &date_modified,
    ]
    .join(" "))
}

fn set_file_name_length(allowed_width: usize, file_name: &str) -> String {
    if file_name.graphemes(true).count() >= allowed_width {
        let file_name_strs = file_name
            .graphemes(true)
            .take(allowed_width)
            .collect::<Vec<&str>>();
        file_name_strs.join("")
    } else {
        let spacer_length = allowed_width - file_name.len();
        let spacer = " ".repeat(spacer_length);
        file_name.to_string() + spacer.as_str()
    }
}

fn get_formatted_date(meta_data: &Metadata, options: TimeOptions) -> String {
    let since_epoch = match options {
        Created => meta_data
            .created()
            .expect(
                "Not anticipated to run on systems that do not implement date created for files",
            )
            .duration_since(UNIX_EPOCH)
            .expect("Clock may have gone backwards"),
        TimeOptions::Modified => meta_data
            .modified()
            .expect(
                "Not anticipated to run on systems that do not implement date modified for files",
            )
            .duration_since(UNIX_EPOCH)
            .expect("Clock may have gone backwards"),
    };
    format_date(since_epoch)
}

fn format_date(since_epoch: Duration) -> String {
    DateTime::<Utc>::from_timestamp(
        since_epoch.as_secs() as i64,
        since_epoch.subsec_nanos(),
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
    use chrono::{DateTime, Utc};
    use std::fs::File;
    use std::io::Write;
    use std::time::SystemTime;
    use std::{fs, thread, time};
    use tempfile::*;
    use unicode_segmentation::UnicodeSegmentation;

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
        };
        let contents = list_contents(&config, 400).unwrap();
        assert!(contents.contains(expected_date.as_str()));
    }

    fn calc_expected_date_string(expected_file_1_created: &SystemTime) -> String {
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
    fn does_not_contain_ext_attrs_headers_when_not_set() {
        let (temp_dir, _file_1, _file_2) = setup_basic_test();
        let config = Config {
            target: temp_dir.path().to_str().unwrap().to_string(),
            to_file: false,
            target_file: "".to_string(),
            extended_attributes: false,
        };
        let contents = list_contents(&config, 400).unwrap();
        assert!(!contents.contains("Date Created"));
        assert!(!contents.contains("Date Modified"));
        assert!(!contents.contains("Permissions"));
    }

    #[test]
    fn does_not_contain_extended_attributes_when_not_set() {
        let (temp_dir, file_1, _file_2) = setup_basic_test();
        let config = Config {
            target: temp_dir.path().to_str().unwrap().to_string(),
            to_file: false,
            target_file: "".to_string(),
            extended_attributes: false,
        };
        let contents = list_contents(&config, 400).unwrap();
        let lines_of_content: Vec<&str> = contents.split('\n').collect();
        let first_file_line = lines_of_content.get(2).unwrap();
        let components: Vec<&str> = first_file_line.split_ascii_whitespace().collect();
        assert_eq!(2, components.len());
    }

    #[test]
    fn file_names_shortened_for_small_terminals_when_ext_attr_set() {
        let (file_1_full_path, compressed_width, target_line) = setup_long_name_test();
        assert_eq!(target_line.len(), compressed_width);
        assert!(!target_line.contains(file_1_full_path.as_str()));
        let expected_content_chars: Vec<&str> = file_1_full_path
            .graphemes(true)
            .take(compressed_width - RESERVED_LENGTH)
            .collect();
        let expected_content = expected_content_chars.join("");
        assert!(target_line.contains(&expected_content));
    }

    fn setup_long_name_test() -> (String, usize, String) {
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
        };
        let file_1_full_path = file_1.to_str().unwrap().to_string();
        let compressed_width = file_1_full_path.graphemes(true).count(); //so always file path is smaller that console
        let contents = list_contents(&config, compressed_width).unwrap();
        let lines_of_content: Vec<&str> = contents.split('\n').collect();
        let first_file_line = lines_of_content.get(2).unwrap();
        let second_file_line = lines_of_content.get(3).unwrap();
        let target_line = if first_file_line.contains("very_long") {
            first_file_line
        } else {
            second_file_line
        };
        (file_1_full_path, compressed_width, target_line.to_string())
    }

    #[test]
    fn there_is_always_space_between_fields() {
        let (_file_1_full_path, _compressed_width, target_line) = setup_long_name_test();
        let n_space_sep_components = target_line.split_ascii_whitespace().count();
        // space between icon and name, name and datec, datec and timec, timec and perm, perm and datem, datem and timem
        assert_eq!(n_space_sep_components, 7);
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
        };
        let inadequate_length = 60; // less than reserved for extended attrs
        let contents = list_contents(&config, inadequate_length).unwrap();
    }

    #[test]
    fn contents_should_align_to_columns() {
        let (temp_dir, file_1, _file_2) = setup_basic_test();
        let config = Config {
            target: temp_dir.path().to_str().unwrap().to_string(),
            to_file: false,
            target_file: "".to_string(),
            extended_attributes: true,
        };
        let contents = list_contents(&config, 120).unwrap();
        let lines: Vec<&str> = contents.split('\n').collect();
        let title_line = lines[0];
        let title_line_words: Vec<&str> = title_line.split("Date").collect();
        let file_name = title_line_words[0];
        let file_name_line = lines[2];
        let file_name_line_words: Vec<&str> = file_name_line.split_ascii_whitespace().collect();
        let file_name_path = file_name_line_words[1];
        assert_eq!(file_name.len(), file_name_path.len() + 3) // for icon & extra space
    }

    #[test]
    fn paths_should_pad_to_max_length() {
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
        };
        let file_1_full_path = file_1.to_str().unwrap().to_string();
        let max_name_width = file_1_full_path.graphemes(true).count();
        let always_sufficient_length = max_name_width + 70; //so always file path is smaller that console
        let contents = list_contents(&config, always_sufficient_length).unwrap();
        let contents_as_lines: Vec<&str> = contents.split('\n').collect();
        let first_path_line = contents_as_lines
            .iter()
            .find(|line| line.contains("very_long_filename"))
            .unwrap();
        let second_path_line = contents_as_lines
            .iter()
            .find(|line| line.contains(FILE_2_NAME))
            .unwrap();
        println!("{}", first_path_line);
        println!("{}", second_path_line);
        assert_eq!(first_path_line.len(), second_path_line.len());
        let expected_file_2_created = file_2.metadata().unwrap().created().unwrap();
        let expected_date_str = calc_expected_date_string(&expected_file_2_created);
        assert!(second_path_line.contains(expected_date_str.as_str()));
        let file_2_parts: Vec<&str> = second_path_line.split(expected_date_str.as_str()).collect();
        let file_1_parts: Vec<&str> = first_path_line.split(expected_date_str.as_str()).collect();
        assert_eq!(FLOPPY_ICON.graphemes(true).count(), 1);
        assert_eq!(file_2_parts[0].len(), file_1_parts[0].len());
    }

    // columns should auto-fit content
}
