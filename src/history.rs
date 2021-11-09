use std::path::{Path, PathBuf};

use xdg::BaseDirectories;

/// A single entry into history, providing the command run and some meta-data
/// describing it.
///
/// todo: add mode
/// todo: add base command
/// todo: serialize entry
pub struct HistoryEntry {
    pub argv: String,
    pub base: Option<String>,
    pub mode: String,
}

impl HistoryEntry {
    /// Construct a new [HistoryEntity] where [argv] contains the contents argv
    /// as a single String, [base] is the wrapped base command if there is one,
    /// and [mode] is the shell execution mode.
    pub fn new(argv: String, base: Option<String>, mode: String) -> HistoryEntry {
        HistoryEntry { argv, base, mode }
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

    file: PathBuf,
}

/// Provides an abstraction around the shell's previously run commands.
///
/// todo: return std::io::Result<HHistory> and add 'create_empty' or 'Default::default()'
/// todo: error on writing history?
/// todo: error on reading history?
/// todo: serialize history
impl History {
    fn find_history_file() -> std::io::Result<PathBuf> {
        let directories = BaseDirectories::new()?;

        let history_file = directories.place_data_file(Path::new("wrash").join("history"))?;

        Ok(history_file)
    }

    /// Creates a new History value using $XDG_DATA_HOME/wrash/history as the
    /// history file. If the file cold not be found or read, the history is
    /// created empty.
    pub fn new() -> History {
        let history_file = Self::find_history_file();
        let history = match history_file {
            Ok(_) => {
                vec![]
            }
            Err(_) => vec![],
        };

        // sample history entries for manual testing
        // let history = vec![
        //     HistoryEntry::new("status".to_string(), Some("git".to_string()), "wrapped".to_string()),
        //     HistoryEntry::new("status docker".to_string(), Some("systemctl".to_string()), "wrapped".to_string()),
        //     HistoryEntry::new("add -A".to_string(), Some("git".to_string()), "wrapped".to_string()),
        //     HistoryEntry::new("commit --message 'some sample commit message'".to_string(), Some("git".to_string()), "wrapped".to_string()),
        //     HistoryEntry::new("ls -l --color auto --group-directories-first".to_string(), None, "normal".to_string()),
        //     HistoryEntry::new("whoami".to_string(), None, "normal".to_string()),
        // ];

        Self {
            history,
            file: history_file.unwrap(),
        }
    }

    pub fn get(&self, index: usize) -> Option<&HistoryEntry> {
        self.history.get(index)
    }

    pub fn get_from_end(&self, index: usize) -> Option<&HistoryEntry> {
        if index >= self.len() {
            None
        } else {
            self.history.get(self.len() - 1 - index)
        }
    }

    pub fn push(&mut self, entry: HistoryEntry) {
        self.history.push(entry);
    }

    /// Sync the current in-memory history with the history file.
    pub fn sync(&self) -> Result<(), std::io::Error> {
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
        }
    }
}

pub struct HistoryIterator<'history> {
    entries: &'history [HistoryEntry],

    index: usize,
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
