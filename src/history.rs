use std::env::join_paths;
use std::path::{Path, PathBuf};

use xdg::BaseDirectories;

// struct HistoryEntry {
//     cmd: String,
//     when: NaiveDateTime,
// }

/// A single entry into history, providing the command run and some meta-data
/// describing it.
///
/// todo: add mode
/// todo: add base command
/// todo: serialize entry
pub struct HistoryEntry<'shell> {
    cmd: String,
    base: &'shell str,
    mode: String,
}

impl HistoryEntry<'_> {
    pub fn new(cmd: String, base: &str, mode: String) -> HistoryEntry {
        HistoryEntry {
            cmd, base, mode
        }
    }
}

pub struct History<'shell> {
    history: Vec<HistoryEntry<'shell>>,

    file: PathBuf,
}

/// Provides an abstraction around the shell's previously run commands.
///
/// todo: return std::io::Result<HHistory> and add 'create_empty' or 'Default::default()'
/// todo: error on writing history?
/// todo: error on reading history?
/// todo: serialize history
impl <'shell> History<'shell> {
    fn find_history_file() -> std::io::Result<PathBuf> {
        let directories = BaseDirectories::new()?;

        let history_file = directories.place_data_file(Path::new("wrash").join("history"))?;

        return Ok(history_file);
    }

    /// Creates a new History value using $XDG_DATA_HOME/wrash/history as the
    /// history file. If the file cold not be found or read, the history is
    /// created empty.
    pub fn new() -> History<'shell> {
        let history_file = Self::find_history_file();
        let history = match history_file {
            Ok(_) => { vec![] },
            Err(_) => vec![],
        };

        Self { history, file: history_file.unwrap() }
    }

    pub fn get(&self, index: usize) -> Option<&HistoryEntry> {
        self.history.get(index)
    }

    pub fn get_from_end(&self, index: usize) -> Option<&HistoryEntry> {
        self.history.get(self.len() - 1 - index)
    }

    pub fn push(&mut self, entry: HistoryEntry<'shell>) {
        self.history.push(entry);
    }

    /// Sync the current in-memory history with the history file.
    pub fn sync(&self) -> Result<(), std::io::Error> { Ok(()) }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}