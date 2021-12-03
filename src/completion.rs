use std::path::PathBuf;
use std::env;

use glob::{self, PatternError};

// todo: each of these methods should allow for filtering executables

/// Search a directory for a paths with the given prefix. If there are multiple
/// matches the shorted available match is returned.
///
/// If any error is encountered while reading a file, that file is ignored.
pub fn search_dir(prefix: &str) -> Result<Vec<PathBuf>, PatternError> {
    let dir_glob = format!("{}*", prefix);

    Ok(glob::glob(dir_glob.as_str())?.filter_map(Result::ok).collect())
}

/// Searches the directories on the system's PATH environment variable for
/// paths with the given prefix. The returned paths are stripped of any parent
/// directories.
///
/// If any error is encountered while reading a file, that file is ignored.
pub fn search_path(prefix: &str) -> Result<Vec<PathBuf>, PatternError> {
    let path_val = env::var("PATH").unwrap_or_else(|_| "".to_string());
    let mut globs: Vec<PathBuf> = vec![];

    for dir in path_val.split(':') {
        let prefix = format!("{}/{}", dir, prefix);
        let found = search_dir(prefix.as_str())?;
        let mut base_names: Vec<PathBuf> = found.iter().map(|path| {
            match path.file_name() {
                Some(base_name) => PathBuf::from(base_name),
                None => path.clone(),
            }
        }).collect();

        globs.append(&mut base_names);
    }

    Ok(globs)
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::env;
    use std::path::PathBuf;
    use tempfile::{self, TempDir};
    use crate::completion;

    #[test]
    fn test_search_dir_empty_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let dir_path = dir.path();

        let a_path = PathBuf::from(dir_path).join("a");
        let b_path = PathBuf::from(dir_path).join("b");
        let c_path = PathBuf::from(dir_path).join("c");
        let d_path = PathBuf::from(&c_path).join("d");

        fs::write(&a_path, "")?;
        fs::write(&b_path, "")?;
        fs::create_dir(&c_path);
        fs::write(&d_path, "")?;

        let actual = completion::search_dir(format!("{}/", dir_path.to_str().unwrap()).as_str())?;
        let expected = vec![
            a_path,
            b_path,
            c_path,
        ];

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn test_search_dir_with_common_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let dir_path = dir.path();

        let a_file_path = PathBuf::from(dir_path).join("a_file");
        let another_path = PathBuf::from(dir_path).join("another_file");
        let directory_path = PathBuf::from(dir_path).join("directory");
        let a_child_path = PathBuf::from(&directory_path).join("a_child");

        fs::write(&a_file_path, "")?;
        fs::write(&another_path, "")?;
        fs::create_dir(&directory_path);
        fs::write(&a_child_path, "")?;

        let actual = completion::search_dir(format!("{}/a", dir_path.to_str().unwrap()).as_str())?;
        let expected = vec![
            a_file_path,
            another_path,
        ];

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn test_on_path() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dirs = vec![tempfile::tempdir()?, tempfile::tempdir()?];

        let new_path = temp_dirs.iter().map(|entry| entry.path().to_str().unwrap()).collect::<Vec<&str>>().join(":");
        env::set_var("PATH", new_path);

        let a_file = temp_dirs[0].path().join("a_file");
        let some_other_file = temp_dirs[0].path().join("some_other_file");
        let another_file = temp_dirs[0].path().join("another_file");

        let another_another_file = temp_dirs[1].path().join("another_another_file");
        let yet_another_file = temp_dirs[1].path().join("yet_another_file");
        let a_final_file = temp_dirs[1].path().join("a_final_file");

        fs::write(a_file, "")?;
        fs::write(some_other_file, "")?;
        fs::write(another_file, "")?;

        fs::write(another_another_file, "")?;
        fs::write(yet_another_file, "")?;
        fs::write(a_final_file, "")?;

        let actual = completion::search_path("a")?;
        let expected = vec![
            PathBuf::from("a_file"),
            PathBuf::from("another_file"),
            PathBuf::from("a_final_file"),
            PathBuf::from("another_another_file"),
        ];

        assert_eq!(expected, actual);

        Ok(())
    }
}