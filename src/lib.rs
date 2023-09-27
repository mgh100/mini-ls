pub mod arg_processing;

use crate::arg_processing::Config;
use std::fs;
use std::fs::{DirEntry, ReadDir};
use std::path::Path;

const FLOPPY: &str = "\u{1F4BE}";
const FOLDER: &str = "\u{1F4C1}";

fn list_contents(directory: &str) -> Result<String, std::io::Error> {
    let dir_read = fs::read_dir(directory);
    match dir_read {
        Ok(file_collection) => Ok(convert_read_dir_to_filename_collection(file_collection)),
        Err(error) => {
            eprintln!("was unable to read the contents of {}", directory);
            Err(error)
        }
    }
}

fn convert_read_dir_to_filename_collection(file_collection: ReadDir) -> String {
    let (directories, files): (Vec<DirEntry>, Vec<DirEntry>) = file_collection
        .into_iter()
        .filter_map(|dir_entry| dir_entry.ok())
        .partition(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()));
    let mut string_list_of_files = prepend_each_entry(files, FLOPPY);
    let mut string_list_of_dirs = prepend_each_entry(directories, FOLDER);
    string_list_of_files.append(&mut string_list_of_dirs);
    string_list_of_files.join("\n")
}

fn prepend_each_entry(dir_entries: Vec<DirEntry>, icon: &str) -> Vec<String> {
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
    let contents = list_contents(config.target.as_str())?;
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

    #[test]
    fn includes_files_inside_folder_in_output() {
        let temp_dir = setup_basic_test();
        let list_of_contents = list_contents(temp_dir.path().to_str().unwrap());
        let list_of_contents = list_of_contents.unwrap();
        assert!(list_of_contents.contains(FILE_1_NAME));
        assert!(list_of_contents.contains(FILE_1_NAME));
    }

    #[test]
    fn includes_that_the_entry_is_a_file() {
        let temp_dir = setup_basic_test();
        let list_of_contents = list_contents(temp_dir.path().to_str().unwrap());
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
        let list_of_contents = list_contents(temp_dir.path().to_str().unwrap());
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
        };
        let contents = list_contents(config.target.as_str());
        assert!(contents.is_err());
    }
}
