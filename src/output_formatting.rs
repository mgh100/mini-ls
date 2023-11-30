use crate::{FileEntryParsingError, FLOPPY, FOLDER, RESERVED_LENGTH};
use std::fs::DirEntry;

pub struct FormattingCommand {
    extended_attr: bool,
    width: usize,
    files: Vec<DirEntry>,
    longest: usize,
}

impl FormattingCommand {
    pub fn new(extended_attr: bool, width: usize, files: Vec<DirEntry>, longest: usize) -> Self {
        FormattingCommand {
            extended_attr,
            width,
            files,
            longest,
        }
    }
}

pub fn generate_textual_display(
    command: FormattingCommand,
    directories: Vec<DirEntry>,
) -> Result<String, FileEntryParsingError> {
    let mut header_row = if command.extended_attr && command.width > 80 {
        crate::create_extended_attr_header(command.width, command.longest)
    } else {
        vec![
            String::from("Name:"),
            String::from("=").repeat(command.width),
        ]
    };
    let mut string_list_of_files = orchestrate_formatting(command)?;
    let mut string_list_of_dirs = crate::format_each_entry(directories, FOLDER)?;
    header_row.append(&mut string_list_of_files);
    header_row.append(&mut string_list_of_dirs);
    Ok(header_row.join("\n"))
}

fn orchestrate_formatting(
    command: FormattingCommand,
) -> Result<Vec<String>, FileEntryParsingError> {
    Ok(if command.extended_attr && command.width > 80 {
        let available_filename_space = command.width - RESERVED_LENGTH;
        let file_name_target_length = if available_filename_space > command.longest {
            command.longest
        } else {
            available_filename_space
        };
        crate::format_each_ext_attr_entry(&command.files, file_name_target_length)?
    } else if command.extended_attr && command.width <= 80 {
        panic!("requires minimum console width of 80");
    } else {
        crate::format_each_entry(command.files, FLOPPY)?
    })
}
