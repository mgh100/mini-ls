pub mod arg_processing;

use crate::arg_processing::Config;
use std::fs;
use std::fs::{DirEntry, ReadDir};
use std::ops::Add;
use std::path::Path;

const FLOPPY: &str = "\u{1F4BE}";
const FOLDER: &str = "\u{1F4C1}";

fn list_contents(config: &Config, width: usize) -> Result<String, std::io::Error> {
    let dir_read = fs::read_dir(&config.target);
    match dir_read {
        Ok(file_collection) => Ok(convert_read_dir_to_filename_collection(
            file_collection,
            config.extended_attributes,
            width,
        )),
        Err(error) => {
            eprintln!("was unable to read the contents of {}", &config.target);
            Err(error)
        }
    }
}

fn convert_read_dir_to_filename_collection(
    file_collection: ReadDir,
    extended_attr: bool,
    width: usize,
) -> String {
    let (directories, files): (Vec<DirEntry>, Vec<DirEntry>) = file_collection
        .into_iter()
        .filter_map(|dir_entry| dir_entry.ok())
        .partition(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()));
    let mut header_row = if extended_attr && width > 80 {
        let date_created_text = "Date Created";
        let date_created_heading = date_created_text
            .to_string()
            .add(" ".repeat(20 - date_created_text.len()).as_str());
        let date_modified_text = "Date Modified";
        let date_modified_heading = date_modified_text
            .to_string()
            .add(" ".repeat(20 - date_modified_text.len()).as_str());
        let permissions_heading = String::from("Permissions ");
        let remaining_width = width - 52;
        let name_text = "Name";
        let owner_text = "Owner";
        let total_file_name_space = (0.7 * remaining_width as f64) as usize;
        let total_owener_space = (0.3 * remaining_width as f64) as usize;
        let name_heading = name_text
            .to_string()
            .add(" ".repeat(total_file_name_space - name_text.len()).as_str());
        let owner_heading = owner_text
            .to_string()
            .add(" ".repeat(total_owener_space - owner_text.len()).as_str());
        let mut header = "".to_string();
        vec![
            header
                + name_heading.as_str()
                + date_created_heading.as_str()
                + owner_heading.as_str()
                + permissions_heading.as_str()
                + date_modified_heading.as_str(),
            String::from("=").repeat(width),
        ]
    } else {
        vec![String::from("Name:"), String::from("=").repeat(width)]
    };
    let mut string_list_of_files = format_each_entry(files, FLOPPY);
    let mut string_list_of_dirs = format_each_entry(directories, FOLDER);
    header_row.append(&mut string_list_of_files);
    header_row.append(&mut string_list_of_dirs);
    header_row.join("\n")
}

fn format_each_entry(dir_entries: Vec<DirEntry>, icon: &str) -> Vec<String> {
    dir_entries
        .into_iter()
        .map(convert_dir_entry_to_str)
        .map(|file_name| icon.to_owned() + " " + &*file_name)
        .collect()
}

fn convert_dir_entry_to_str(dir_entry: DirEntry) -> String {
    let file_name = dir_entry.file_name();
    let normal_str = file_name.to_str().unwrap();
    String::from(normal_str)
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
    use std::fs;
    use std::fs::File;
    use tempfile::*;

    const FILE_1_NAME: &str = "file_1.txt";
    const FILE_2_NAME: &str = "file_2.txt";
    const FLOPPY_ICON: &str = "\u{1F4BE}";
    const FOLDER_ICON: &str = "\u{1F4C1}";

    fn setup_basic_test() -> TempDir {
        let temp_dir = tempdir().unwrap();
        let file_1 = temp_dir.path().join(FILE_1_NAME);
        let file_2 = temp_dir.path().join(FILE_2_NAME);
        File::create(&file_1).unwrap();
        File::create(&file_2).unwrap();
        assert!(file_1.as_path().exists());
        assert!(file_2.as_path().exists());
        temp_dir
    }

    fn get_typical_config(dir: Option<TempDir>) -> (Config, TempDir) {
        let temp_dir = if let Some(dir_arg) = dir {
            dir_arg
        } else {
            setup_basic_test()
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
        let temp_dir = setup_basic_test();
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
        let temp_dir = setup_basic_test();
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
        let temp_dir = setup_basic_test();
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
        assert!(!contents.contains("Owner"));
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
        let temp_dir = setup_basic_test();
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
        assert!(contents.contains("Owner"));
        assert!(contents.contains("Permissions"));
    }

    #[test]
    fn spaces_out_columns() {
        let temp_dir = setup_basic_test();
        let config = Config {
            target: temp_dir.path().to_str().unwrap().to_string(),
            to_file: false,
            target_file: "".to_string(),
            extended_attributes: true,
        };
        // Date Created and Date Modified = 20 each, Permissions = 12 (word length only), divide rest 70/30 Name/Owner
        let expected_header = "Name                             Date Created        Owner         Permissions Date Modified       ";
        let contents = list_contents(&config, 100).unwrap();
        let lines_of_content: Vec<&str> = contents.split('\n').collect();
        let header = lines_of_content[0];
        assert_eq!(expected_header, header);
    }

    //contains rows that include the extended attributes when true
    //rows do not contain extended attributes when false
    //long file names are shortened for small widths to maintain extended attributes
    //spaces inserted between fields match intended widths of each column
    //all fields end with one space even if overflowed
}
