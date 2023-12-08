use crate::FileEntryParsingError::UnableToCalculatePathLengths;
use crate::TimeOptions::{Created, Modified};
use crate::{FileEntryParsingError, TimeOptions};
use chrono::{DateTime, Utc};
use std::fs::{DirEntry, Metadata};
use std::ops::Add;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, UNIX_EPOCH};
use unicode_segmentation::UnicodeSegmentation;

pub const FLOPPY: &str = "\u{1F4BE}";
const FOLDER: &str = "\u{1F4C1}";
pub const RESERVED_LENGTH: usize = 66;
pub const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

pub struct FormattingCommand {
    extended_attr: bool,
    width: usize,
    files: Vec<DirEntry>,
    directories: Vec<DirEntry>,
}

impl FormattingCommand {
    pub fn new(
        extended_attr: bool,
        width: usize,
        files: Vec<DirEntry>,
        directories: Vec<DirEntry>,
    ) -> Self {
        FormattingCommand {
            extended_attr,
            width,
            files,
            directories,
        }
    }
}

pub fn generate_textual_display(
    command: FormattingCommand,
) -> Result<String, FileEntryParsingError> {
    let boxed_command = Rc::new(command);
    let command_ref = boxed_command.as_ref();
    let Some(longest) = analyse_longest(boxed_command.as_ref()) else {
        return Err(UnableToCalculatePathLengths);
    };
    let mut header_row = if command_ref.extended_attr && command_ref.width > 80 {
        create_extended_attr_header(command_ref.width, longest)
    } else {
        vec![
            String::from("Name:"),
            String::from("=").repeat(command_ref.width),
        ]
    };
    let mut string_list_of_files = orchestrate_formatting(boxed_command.as_ref(), longest)?;
    let mut string_list_of_dirs = format_each_entry(&command_ref.directories, FOLDER)?;
    header_row.append(&mut string_list_of_files);
    header_row.append(&mut string_list_of_dirs);
    Ok(header_row.join("\n"))
}

fn analyse_longest(command: &FormattingCommand) -> Option<usize> {
    let joined = [&command.files, &command.directories];
    let full_list: Vec<&DirEntry> = joined.iter().flat_map(|vec| vec.iter()).collect();
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

fn create_extended_attr_header(width: usize, longest: usize) -> Vec<String> {
    let date_created_heading = create_heading_of_width(24usize, "Date Created");
    let date_modified_heading = create_heading_of_width(24usize, "Date Modified");
    let permissions_heading = create_heading_of_width(13usize, "Permissions");
    let remaining_width = if longest + 4 <= width - 60 {
        longest + 4
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

fn create_heading_of_width(head_width: usize, name: &str) -> String {
    name.to_string().add(
        " ".repeat(head_width - name.graphemes(true).count())
            .as_str(),
    )
}

fn orchestrate_formatting(
    command: &FormattingCommand,
    longest: usize,
) -> Result<Vec<String>, FileEntryParsingError> {
    Ok(if command.extended_attr && command.width > 80 {
        let available_filename_space = command.width - RESERVED_LENGTH;
        let file_name_target_length = if available_filename_space > longest {
            longest
        } else {
            available_filename_space
        };
        format_each_ext_attr_entry(&command.files, file_name_target_length)?
    } else if command.extended_attr && command.width <= 80 {
        panic!("requires minimum console width of 80");
    } else {
        format_each_entry(&command.files, FLOPPY)?
    })
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
        "read only   "
    } else {
        "writable    "
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
    dir_entries: &Vec<DirEntry>,
    icon: &str,
) -> Result<Vec<String>, FileEntryParsingError> {
    Ok(dir_entries
        .iter()
        .filter_map(|entry| convert_dir_entry_to_str(entry).ok())
        .map(|file_name| icon.to_owned() + " " + &file_name)
        .collect())
}

fn convert_dir_entry_to_str(dir_entry: &DirEntry) -> Result<String, FileEntryParsingError> {
    let file_name = dir_entry.file_name();
    let normal_str = match file_name.to_str() {
        Some(name) => name,
        None => return Err(FileEntryParsingError::FileNameInvalidUnicode),
    };
    Ok(String::from(normal_str))
}

#[cfg(test)]
mod tests {
    use crate::output_formatting::{generate_textual_display, FormattingCommand, FOLDER};
    use std::fs;
    use std::fs::{DirEntry, File};
    use tempfile::tempdir;

    fn setup_test() -> (Vec<DirEntry>, Vec<DirEntry>) {
        const FILE_1_NAME: &str = "file_1.txt";
        const FILE_2_NAME: &str = "file_2.txt";

        let temp_dir = tempdir().unwrap();
        let file_1 = temp_dir.path().join(FILE_1_NAME);
        let file_2 = temp_dir.path().join(FILE_2_NAME);
        let extra_dir = temp_dir.path().join("other");
        File::create(&file_1).unwrap();
        File::create(&file_2).unwrap();
        fs::create_dir(&extra_dir).unwrap();
        let dir_read = fs::read_dir(temp_dir.path()).unwrap();

        dir_read
            .filter_map(|entry| entry.ok())
            .partition(|entry| entry.metadata().unwrap().is_file())
    }

    #[test]
    fn non_extended_output_contains_header_row() {
        let (file_entries, directories) = setup_test();
        let command = FormattingCommand::new(false, 200, file_entries, directories);
        let content = generate_textual_display(command).unwrap();
        let lines_of_content = content.split('\n').collect::<Vec<&str>>();
        let header_row = lines_of_content.get(0).unwrap();
        assert!(header_row.starts_with("Name"));
        assert!(!header_row.contains("Date Created"));
        assert!(!header_row.contains("Date Modified"));
        assert!(!header_row.contains("Permissions"));
    }

    #[test]
    fn includes_folder_icon_for_sub_folders() {
        let (file_entries, directories) = setup_test();
        let command = FormattingCommand::new(false, 100, file_entries, directories);
        let content = generate_textual_display(command).unwrap();
        assert_eq!(
            content
                .lines()
                .filter(|line| line.starts_with(FOLDER))
                .count(),
            1
        );
    }
}
