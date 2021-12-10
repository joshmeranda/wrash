use faccess::PathExt;
use std::path::{Component, Path, PathBuf};

use glob::{self, PatternError};

// todo: handle duplicates

/// Merge the prefix path with the completion path to restore any path
/// component lost during processing.
fn merge_prefix_with_completion(original_path: &Path, new: &Path) -> Option<PathBuf> {
    if let Some(first) = original_path.components().next() {
        if matches!(first, Component::CurDir) {
            let merged: PathBuf = vec![first.as_os_str(), new.as_os_str()].iter().collect();

            return Some(merged)
        }
    }

    None
}

/// Search the file system for paths with a given prefix allowing for wildcards. The returns
/// `pathBuf`s are normalized (meaning any `.` and `..` are stripped out.
///
/// If any error is encountered while reading a file, that file is ignored.
pub fn search_prefix(prefix: &Path) -> Result<impl Iterator<Item = PathBuf>, PatternError> {
    let prefix_path = PathBuf::from(format!("{}*", prefix.to_str().unwrap()));

    Ok(glob::glob(prefix_path.to_str().unwrap())?.
        filter_map(Result::ok)
        .map(move |p| if let Some(merged) = merge_prefix_with_completion(prefix_path.as_path(), p.as_path()) {
            merged
        } else {
            p
        }))
}

/// Searches the directories on the system's PATH environment variable for
/// paths with the given prefix. The returned paths are stripped of any parent
/// directories.
///
/// If any error is encountered while reading a file, that file is ignored.
pub fn search_path<'a>(
    prefix: &'a Path,
    path_val: &'a str,
) -> Result<impl Iterator<Item = PathBuf> + 'a, PatternError> {
    let globs = path_val
        .split(':')
        .map(move |dir: &str| {
            let full_prefix = Path::new(dir).join(prefix);

            // todo: handle pattern errors
            let found = search_prefix(full_prefix.as_path())
                .unwrap()
                .into_iter()
                .filter(|path| !path.is_dir() && path.executable())
                .map(|path| match path.components().last() {
                    Some(component) => PathBuf::from(component.as_os_str()),
                    None => path,
                });

            found
        })
        .flatten();

    Ok(globs)
}

#[cfg(test)]
mod test {
    use std::{env, path};
    use crate::completion;
    use std::path::{Path, PathBuf};

    fn get_resource_path(components: &[&str]) -> PathBuf {
        vec!["tests", "resources"].iter().chain(components.iter()).collect()
    }

    fn push_trailing_slash(p: PathBuf) -> PathBuf {
        let mut s = p.to_string_lossy().to_string();

        s.push(path::MAIN_SEPARATOR);

        PathBuf::from(s)
    }

    #[test]
    fn test_search_dir_empty_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let dir_path = push_trailing_slash(get_resource_path(&["a_directory"]));

        let mut actual =
            completion::search_prefix(dir_path.as_path())?;

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

        let mut actual = completion::search_prefix(dir_path.join("a").as_path())?;

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
    fn test_search_dir_with_directory() -> Result<(), Box<dyn std::error::Error>> {
        let prefix_path = get_resource_path(&["a_directory", "directory", "a"]);

        let mut actual = completion::search_prefix(prefix_path.as_path())?;

        assert_eq!(Some(get_resource_path(&["a_directory", "directory", "a_child"])), actual.next());
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

        let mut actual = completion::search_path(Path::new("a"), new_path.as_str())?;

        assert_eq!(Some(PathBuf::from("a_file")), actual.next());
        assert_eq!(Some(PathBuf::from("a_final_file")), actual.next());
        assert_eq!(None, actual.next());

        Ok(())
    }

    #[ignore]
    #[test]
    fn test_search_dir_for_executable_in_cwd() -> Result<(), Box<dyn std::error::Error>> {
        let old_cwd = env::current_dir()?;
        let new_cwd = get_resource_path(&["a_directory", "directory"]);

        env::set_current_dir(new_cwd)?;
        let mut actual = completion::search_prefix(Path::new(".").join("a").as_path())?;

        assert_eq!(Some(Path::new(".").join("a_child")), actual.next());
        assert_eq!(None, actual.next());

        env::set_current_dir(old_cwd)?;

        Ok(())
    }
}
