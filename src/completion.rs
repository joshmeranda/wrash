use faccess::PathExt;
use std::path::PathBuf;

use glob::{self, PatternError};

// todo: add common prefix finder
// todo: handle duplicates

/// Search a directory for a paths with the given prefix. If there are multiple
/// matches the shorted available match is returned.
///
/// If any error is encountered while reading a file, that file is ignored.
pub fn search_dir(prefix: &str) -> Result<impl Iterator<Item = PathBuf>, PatternError> {
    let dir_glob = format!("{}*", prefix);

    Ok(glob::glob(dir_glob.as_str())?.filter_map(Result::ok))
}

/// Searches the directories on the system's PATH environment variable for
/// paths with the given prefix. The returned paths are stripped of any parent
/// directories.
///
/// If any error is encountered while reading a file, that file is ignored.
pub fn search_path<'a>(
    prefix: &'a str,
    path_val: &'a str,
) -> Result<impl Iterator<Item = PathBuf> + 'a, PatternError> {
    let globs = path_val
        .split(':')
        .map(move |dir: &str| {
            // todo: user PathUBf::join to use the right path separator
            let full_prefix = format!("{}/{}", dir.to_string(), prefix);

            // todo: handle pattern errors
            let found = search_dir(full_prefix.as_str())
                .unwrap()
                .into_iter()
                .filter(|path| !path.is_dir() && path.executable())
                .map(|path| match path.file_name() {
                    Some(base_name) => PathBuf::from(base_name),
                    None => path,
                });

            found
        })
        .flatten();

    Ok(globs)
}

#[cfg(test)]
mod test {
    use crate::completion;
    use std::path::PathBuf;

    fn get_resource_path(components: &[&str]) -> PathBuf {
        components.iter().fold(
            PathBuf::from("tests").join("resources"),
            |acc, component| acc.join(component),
        )
    }

    #[test]
    fn test_search_dir_empty_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let dir_path = get_resource_path(&["a_directory"]);

        let mut actual =
            completion::search_dir(format!("{}/", dir_path.to_str().unwrap()).as_str())?;

        assert_eq!(
            Some(get_resource_path(&["a_directory", "a_file"])),
            actual.next()
        );
        assert_eq!(
            Some(get_resource_path(&["a_directory", "another_file"])),
            actual.next()
        );
        assert_eq!(
            Some(get_resource_path(&["a_directory", "directory"])),
            actual.next()
        );
        assert_eq!(
            Some(get_resource_path(&["a_directory", "some_other_file"])),
            actual.next()
        );
        assert_eq!(None, actual.next());

        Ok(())
    }

    #[test]
    fn test_search_dir_with_common_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let dir_path = get_resource_path(&["a_directory"]);

        let mut actual =
            completion::search_dir(format!("{}/a", dir_path.to_str().unwrap()).as_str())?;

        assert_eq!(
            Some(get_resource_path(&["a_directory", "a_file"])),
            actual.next()
        );
        assert_eq!(
            Some(get_resource_path(&["a_directory", "another_file"])),
            actual.next()
        );
        assert_eq!(None, actual.next());

        Ok(())
    }

    #[test]
    fn test_on_path() -> Result<(), Box<dyn std::error::Error>> {
        let new_path = vec![
            get_resource_path(&["a_directory"]),
            get_resource_path(&["some_other_directory"]),
        ]
        .iter()
        .map(|entry| entry.to_str().unwrap())
        .collect::<Vec<&str>>()
        .join(":");

        let mut actual = completion::search_path("a", new_path.as_str())?;

        assert_eq!(Some(PathBuf::from("a_file")), actual.next());
        assert_eq!(Some(PathBuf::from("a_final_file")), actual.next());
        assert_eq!(None, actual.next());

        Ok(())
    }
}
