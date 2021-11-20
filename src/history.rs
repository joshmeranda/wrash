use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_yaml;

use xdg::BaseDirectories;

use crate::session::SessionMode;

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

pub struct History {
    history: Vec<HistoryEntry>,

    path: PathBuf,
}

/// Provides an abstraction around the shell's previously run commands.
impl History {
    fn find_history_file() -> Option<PathBuf> {
        match BaseDirectories::new() {
            Ok(directories) => {
                Some(directories.get_data_file(Path::new("wrash").join("history.yaml")))
            }
            Err(_) => None,
        }
    }

    /// Creates a new History value using $XDG_DATA_HOME/wrash/history as the
    /// history file. If the file cold not be found or read, the history is
    /// created empty.
    pub fn new() -> Result<History, String> {
        let path = match Self::find_history_file() {
            Some(path) => path,
            None => {
                return Err("could not determine a home directory for the current user".to_string())
            }
        };

        let history: Vec<HistoryEntry> = if path.exists() {
            let s = match fs::read_to_string(path.as_path()) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("Error: could not read file: {}", err);
                    return Err(format!("could not read file: {}", err));
                }
            };

            // todo: handle deserialization errors
            serde_yaml::from_str(s.as_str()).unwrap()
        } else {
            vec![]
        };

        // sample history entries for manual testing
        let history = vec![
            HistoryEntry::new("history".to_string(), None, SessionMode::Normal, true),
            HistoryEntry::new(
                "status docker".to_string(),
                Some("systemctl".to_string()),
                SessionMode::Wrapped,
                false,
            ),
            HistoryEntry::new(
                "commit --message 'some sample commit message'".to_string(),
                Some("git".to_string()),
                SessionMode::Wrapped,
                false,
            ),
            HistoryEntry::new(
                "ls -l --color auto --group-directories-first".to_string(),
                None,
                SessionMode::Normal,
                false,
            ),
            HistoryEntry::new("whoami".to_string(), None, SessionMode::Normal, false),
        ];

        Ok(Self { history, path })
    }

    pub fn push(&mut self, entry: HistoryEntry) {
        self.history.push(entry);
    }

    /// Sync the current in-memory history with the history file.
    pub fn sync(&self) -> Result<(), std::io::Error> {
        let s = serde_yaml::to_string(self.history.as_slice())
            .expect("to-string should not have erred");
        let mut history_file = File::create(self.path.as_path())?;

        write!(history_file, "{}", s)?;

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
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
