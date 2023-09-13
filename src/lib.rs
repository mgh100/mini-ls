

pub mod runner {
    use std::fs;
    use std::fs::{DirEntry, ReadDir};

    pub fn list_contents(directory: &str) -> String {
        let dir_read = fs::read_dir(directory);
        match dir_read {
            Ok(file_collection) => convert_read_dir_to_filename_collection(file_collection),
            Err(error) => panic!("Could not read directory: {}", error)
        }
    }

    fn convert_read_dir_to_filename_collection(file_collection: ReadDir) -> String {
        let file_name_list: Vec<String> = file_collection.into_iter()
          .filter(|dir_entry| dir_entry.is_ok())
          .map(|dir_entry| dir_entry.expect("already checked"))
          .map(convert_dir_entry_to_str)
          .collect();
        file_name_list.join("\n")
    }

    fn convert_dir_entry_to_str(dir_entry: DirEntry) -> String {
        let file_name = dir_entry.file_name();
        let normal_str = file_name.to_str().unwrap();
        String::from(normal_str)
    }
}


#[cfg(test)]
mod tests {
    use tempfile::*;
    use std::fs::File;
    use crate::runner::*;

    #[test]
    fn includes_files_inside_folder_in_output() {
        const FILE_1_NAME: &str = "file_1.txt";
        const FILE_2_NAME: &str = "file_2.txt";
        let temp_dir = tempdir().unwrap();
        let file_1 = temp_dir.path().join(FILE_1_NAME);
        let file_2 = temp_dir.path().join(FILE_2_NAME);
        File::create(&file_1).unwrap();
        File::create(&file_2).unwrap();
        assert!(file_1.as_path().exists());
        assert!(file_2.as_path().exists());
        let list_of_contents = list_contents(temp_dir.path().to_str().unwrap());
        assert!(list_of_contents.contains(FILE_1_NAME));
        assert!(list_of_contents.contains(FILE_1_NAME));
    }
}
