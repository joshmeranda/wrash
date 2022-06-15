use std::fs::{self, File};
use std::io::{ErrorKind, Write};
use std::path::PathBuf;

use crate::session::SessionMode;
use crate::WrashError;

/// A single entry into history, providing the command run and some meta-data
/// describing it.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct HistoryEntry {
    pub argv: String,
    pub base: Option<String>,
    pub mode: SessionMode,
    pub is_builtin: bool,
}

impl HistoryEntry {
    /// Construct a new [HistoryEntity] where [argv] contains the contents argv
    /// as a single String, [base] is the wrapped base command if there is one,
    /// and [mode] is the shell execution mode.
    pub fn new(
        argv: String,
        base: Option<String>,
        mode: SessionMode,
        is_builtin: bool,
    ) -> HistoryEntry {
        HistoryEntry {
            argv,
            base,
            mode,
            is_builtin,
        }
    }

    pub fn get_command(&self) -> String {
        match self.base.clone() {
            Some(base) => format!("{} {}", base, self.argv.clone()),
            None => self.argv.clone(),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct History {
    history: Vec<HistoryEntry>,

    // ideally would  be an &Path rather than PathBuf
    path: Option<PathBuf>,
}

/// Provides an abstraction around the shell's previously run commands.
impl History {
    fn find_history_file() -> Option<PathBuf> {
        dirs::data_dir().map(|p| p.join("wrash").join("history.yaml"))
    }

    /// Creates a new `History` value using $XDG_DATA_HOME/wrash/history as the
    /// history file.
    pub fn new() -> Result<History, WrashError> {
        match History::find_history_file() {
            Some(path) => History::with_file(path),
            None => Err(WrashError::FailedIo(std::io::Error::new(
                std::io::ErrorKind::Other,
                "could not determine where to look for the user's history file",
            ))),
        }
    }

    /// Construct a new `History` file using the given file as the source. If
    /// the file cold not be found or read, the history is created empty, and
    /// the target file is also created.
    fn with_file(path: PathBuf) -> Result<History, WrashError> {
        let s = match fs::read_to_string(path.as_path()) {
            Ok(s) => s,
            Err(err) => return Err(WrashError::FailedIo(err)),
        };

        let history = if s.is_empty() {
            vec![]
        } else {
            match serde_yaml::from_str(s.as_str()) {
                Ok(history) => history,
                Err(err) => return Err(WrashError::Custom(err.to_string())),
            }
        };

        Ok(Self {
            history,
            path: Some(path),
        })
    }

    pub fn empty() -> History {
        History {
            history: vec![],
            path: None,
        }
    }

    pub fn push(&mut self, entry: HistoryEntry) {
        self.history.push(entry);
    }

    /// Sync the current in-memory history with the history file.
    ///
    /// If the history is stored in memory only (self.path == None), this
    /// method returns an error.
    pub fn sync(&self) -> Result<(), WrashError> {
        if self.path.is_none() {
            return Err(WrashError::FailedIo(std::io::Error::new(
                std::io::ErrorKind::Other,
                "no history file exists for struct instance",
            )));
        }

        let s = serde_yaml::to_string(self.history.as_slice())
            .expect("to-string should not have erred");

        let mut history_file = match File::create(self.path.as_ref().unwrap().as_path()) {
            Ok(f) => f,
            Err(err) => match err.kind() {
                ErrorKind::NotFound => {
                    if let Some(parent) = self.path.as_ref().unwrap().parent() {
                        fs::create_dir_all(parent)?;
                    }

                    File::create(self.path.as_ref().unwrap().as_path())?
                }
                _ => return Err(WrashError::FailedIo(err)),
            },
        };

        write!(history_file, "{}", s)?;

        Ok(())
    }

    pub fn iter(&self) -> HistoryIterator {
        HistoryIterator {
            entries: self.history.as_slice(),
            index: 0,
            back_index: self.history.len(),
        }
    }
}

pub struct HistoryIterator<'history> {
    entries: &'history [HistoryEntry],

    index: usize,

    back_index: usize,
}

impl<'history> Iterator for HistoryIterator<'history> {
    type Item = &'history HistoryEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.entries.get(self.index);

        if entry.is_some() {
            self.index += 1;
        }

        entry
    }
}

impl<'history> DoubleEndedIterator for HistoryIterator<'history> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.back_index == 0 {
            None
        } else {
            self.back_index -= 1;

            self.entries.get(self.back_index)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::history::HistoryEntry;
    use crate::{History, SessionMode, WrashError};
    use std::fs::read_to_string;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn get_resource_path(components: &[&str]) -> PathBuf {
        vec!["tests", "resources"]
            .iter()
            .chain(components.iter())
            .collect()
    }

    #[test]
    fn test_with_file() -> Result<(), Box<dyn std::error::Error>> {
        let history_path = get_resource_path(&["history", "history.yaml"]);

        let expected = History {
            history: vec![
                HistoryEntry::new(
                    "subcmd -arg 1 -arg 2".to_string(),
                    Some("cmd".to_string()),
                    SessionMode::Wrapped,
                    false,
                ),
                HistoryEntry::new(
                    "othersubcmd --verbose ARG".to_string(),
                    None,
                    SessionMode::Normal,
                    false,
                ),
                HistoryEntry::new("mode".to_string(), None, SessionMode::Wrapped, true),
            ],
            path: Some(history_path.clone()),
        };
        let actual = History::with_file(history_path)?;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn test_with_file_no_exist() -> Result<(), Box<dyn std::error::Error>> {
        let history_path = get_resource_path(&["history", "i do not exist"]);

        let expected = Err(WrashError::FailedIo(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "",
        )));
        let actual = History::with_file(history_path);

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn test_with_file_bad_syntax() -> Result<(), Box<dyn std::error::Error>> {
        let history_path = get_resource_path(&["history", "history.invalid.yaml"]);

        let expected = Err(WrashError::Custom(".[0].is_builtin: invalid type: string \"false,\", expected a boolean at line 3 column 15".to_string()));
        let actual = History::with_file(history_path);

        assert_eq!(expected, actual);

        Ok(())
    }
}
