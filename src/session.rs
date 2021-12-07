use std::cmp::Ordering;
use std::env;
use std::fmt::{Display, Formatter};
use std::io::{self, Write};
use std::path::{self, Path, PathBuf};
use std::str::FromStr;

use termion::clear::{AfterCursor, All};
use termion::cursor::{Goto, Restore, Right, Save};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use faccess::PathExt;

use crate::completion;
use crate::history::{History, HistoryEntry, HistoryIterator};

use crate::prompt;

/// Get the position in a string at which the current word begins.
///
/// todo: handle escaped spaces (ie '\ ')
fn get_word_start(buffer: &str, cursor_offset: usize) -> usize {
    let mut position = cursor_offset;


    while position > 0 && buffer.chars().nth(position - 1).unwrap() != ' ' {
        position -= 1;
    }

    position
}

/// Get the tab completion values.
fn get_tab_completions(prefix: &str, is_command: bool) -> Vec<String> {
    let prefix_path = Path::new(prefix);

    let has_parent = ! prefix.is_empty()
        && prefix[prefix.len() - 1..] == path::MAIN_SEPARATOR.to_string()
        || prefix_path.parent().map_or(false, |parent| ! parent.as_os_str().is_empty());

    if is_command {
        // if the prefix has a parent component, search for directories or executables
        if has_parent {
            return completion::search_dir(prefix).unwrap()
                .filter(|path| path.executable())
                .map(|path| format!("{}{}", prefix, path.to_string_lossy().to_string()))
                .collect();
        }

        // if the prefix does not have a parent component, search on path or directories
        let path_var = env::var("PATH").unwrap_or_else(|_| "".to_string());
        let in_path = completion::search_path(prefix, path_var.as_str())
            .unwrap()
            .map(|path| path.to_string_lossy().to_string());

        let in_dir = completion::search_dir(prefix).unwrap()
            .filter(|path| path.is_dir())
            .map(|path| path.to_string_lossy().to_string());

        in_dir.chain(in_path).collect()
    } else {
        completion::search_dir(prefix).unwrap()
            .map(|path| {
                match path.file_name() {
                    Some(base_name) => PathBuf::from(base_name),
                    None => path,
                }.to_string_lossy().to_string()
            })
            .collect()
    }
}

/// Enum describing the current session execution mode.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum SessionMode {
    Wrapped,
    Normal,
}

impl FromStr for SessionMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "wrapped" => Ok(SessionMode::Wrapped),
            "normal" => Ok(SessionMode::Normal),
            _ => Err(()),
        }
    }
}

impl Display for SessionMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SessionMode::Wrapped => "wrapped",
            SessionMode::Normal => "normal",
        };

        write!(f, "{}", s)
    }
}

// todo: ass support for frozen mode (cannot use `setmode` to change the shell session mode to normal)
pub struct Session<'shell> {
    history: History,

    pub base: &'shell str,

    pub mode: SessionMode,
}

impl<'shell> Session<'shell> {
    pub fn new(history: History, base: &'shell str, mode: SessionMode) -> Session<'shell> {
        Session {
            history,
            base,
            mode,
        }
    }

    pub fn get_mode(&self) -> SessionMode {
        self.mode
    }

    pub fn get_base(&self) -> String {
        self.base.to_string()
    }

    /// Take user input.
    ///
    /// todo: handle returning terminal mode to normal when session is in normal mode
    /// todo: consider a callback architecture to make it easier to reset tab_is_hit
    pub fn take_input(&mut self) -> Result<String, io::Error> {
        let stdout = io::stdout();
        let mut stdout = stdout.lock().into_raw_mode().unwrap();

        let stdin = io::stdin();
        let stdin = stdin.lock();

        let mut buffer = String::new();

        let mut offset = 0usize;

        let history_entries: Vec<&HistoryEntry> = self
            .history
            .iter()
            .filter(|entry| {
                entry.is_builtin
                    || (entry.mode == self.mode
                        && (entry.base.is_none() || entry.base.as_ref().unwrap() == self.base))
            })
            .rev()
            .collect();
        let mut history_offset: Option<usize> = None;
        let mut buffer_bak: Option<String> = None;

        let prompt = prompt();

        write!(stdout, "{}{}", Save, prompt)?;
        stdout.flush()?;

        write!(stdout, "{}", Right(1))?;

        // todo: implement some tab-completion (even if its just files)
        for key in stdin.keys() {
            match key.unwrap() {// character deletion
                Key::Backspace => {
                    if offset > 0 {
                        buffer.remove(offset - 1);
                        offset -= 1;
                    }
                }
                Key::Delete => {
                    if offset < buffer.len() {
                        buffer.remove(offset);
                    }
                }

                // cursor movement
                Key::Left => {
                    if offset != 0 {
                        offset -= 1;
                    }
                }
                Key::Right => {
                    if offset < buffer.len() {
                        offset += 1;
                    }
                }

                Key::Up => {
                    match history_offset {
                        Some(n) => {
                            if n + 1 < history_entries.len() {
                                history_offset = Some(n + 1);
                            }
                        }
                        None => {
                            history_offset = Some(0);
                            buffer_bak = Some(buffer.clone());
                        }
                    };

                    if let Some(entry) = history_entries.get(history_offset.unwrap()) {
                        if entry.mode == SessionMode::Wrapped && !entry.is_builtin {
                            buffer = entry.argv.clone();
                        } else {
                            buffer = entry.get_command();
                        }

                        offset = buffer.len();
                    }
                }
                Key::Down => {
                    if let Some(n) = history_offset {
                        match n.cmp(&0usize) {
                            Ordering::Greater => history_offset = Some(n - 1),
                            Ordering::Equal => {
                                history_offset = None;

                                buffer = buffer_bak.unwrap();
                                buffer_bak = None;
                            }
                            Ordering::Less => unreachable!(),
                        }
                    }

                    if let Some(history_offset) = history_offset {
                        if let Some(entry) = history_entries.get(history_offset) {
                            if entry.mode == SessionMode::Wrapped && !entry.is_builtin {
                                buffer = entry.argv.clone();
                            } else {
                                buffer = entry.get_command();
                            }
                        }
                    }

                    offset = buffer.len();
                }

                // content deletion
                Key::Ctrl('u') => {
                    buffer.replace_range(..offset, "");
                    offset = 0;
                }
                Key::Ctrl('k') => buffer.replace_range(offset.., ""),

                // cursor control
                Key::Ctrl('a') => offset = 0,
                Key::Ctrl('e') => offset = buffer.len(),

                // screen control
                // todo: write lines and scroll rather than clearing screen
                Key::Ctrl('l') => {
                    write!(
                        stdout,
                        "{}{}{}{}{}",
                        Restore,
                        All,
                        Right(offset as u16),
                        Goto(1, 1),
                        Save
                    )?
                },

                // tab completion
                Key::Char('\t') => {
                    // todo: if multiple matches save matches and wait for second TAB
                    // todo: is single match insert into the buffer
                }

                Key::Char('\n') => break,
                Key::Char(c) => {
                    if offset == buffer.len() {
                        buffer.push(c);
                    } else {
                        buffer.insert(offset, c);
                    }

                    offset += 1;
                }

                _ => { /* do nothing */ }
            };

            write!(
                stdout,
                "{}{}{}{}{}{}",
                Restore,
                AfterCursor,
                prompt,
                buffer,
                Restore,
                Right((prompt.len() + offset) as u16)
            )?;

            stdout.flush()?;
        }

        writeln!(stdout, "{}", Restore)?;
        stdout.flush()?;

        Ok(buffer)
    }

    /// Push the given command to the back of the in-memory history stack.
    ///
    /// If the given command is a builtin, it will be added as having no bas
    /// command and SessionMode::Normal.
    pub fn push_to_history(&mut self, command: &str, is_builtin: bool) {
        let entry = if is_builtin {
            HistoryEntry::new(command.trim().to_string(), None, self.mode, true)
        } else {
            match self.mode {
                SessionMode::Wrapped => HistoryEntry::new(
                    command.trim().to_string(),
                    Some(self.get_base()),
                    self.mode,
                    is_builtin,
                ),
                SessionMode::Normal => {
                    HistoryEntry::new(command.trim().to_string(), None, self.mode, false)
                }
            }
        };

        self.history.push(entry);
    }

    pub fn history_iter(&self) -> HistoryIterator {
        self.history.iter()
    }

    pub fn history_sync(&self) -> Result<(), std::io::Error> {
        self.history.sync()
    }
}

impl Drop for Session<'_> {
    fn drop(&mut self) {
        if let Err(err) = self.history_sync() {
            eprintln!(
                "Error: could not write session history to history file: {}",
                err
            );
        }
    }
}

#[cfg(test)]
mod test_get_word_start {
    use crate::session;

    #[test]
    fn get_word_start_single_from_end() {
        let buffer = "word";
        let offset = buffer.len();

        let expected = 0;
        let actual = session::get_word_start(buffer, offset);

        assert_eq!(expected, actual);
    }

    #[test]
    fn get_word_start_single_from_middle() {
        let buffer = "word";
        let offset = buffer.len() / 2;

        let expected = 0;
        let actual = session::get_word_start(buffer, offset);

        assert_eq!(expected, actual);
    }

    #[test]
    fn get_word_start_single_from_starr() {
        let buffer = "word";
        let offset = 0;

        let expected = 0;
        let actual = session::get_word_start(buffer, offset);

        assert_eq!(expected, actual);
    }

    #[test]
    fn get_word_start_multiple_last_word() {
        let buffer = "some example words";
        let offset = buffer.len();

        let expected = 13;
        let actual = session::get_word_start(buffer, offset);

        assert_eq!(expected, actual);
    }

    #[test]
    fn get_word_start_multiple_last_word_from_start() {
        let buffer = "some example words";
        let offset = 13;

        let expected = 13;
        let actual = session::get_word_start(buffer, offset);

        assert_eq!(expected, actual);
    }

    #[ignore] // todo: support for escaped whitespace is not yet implemented
    #[test]
    fn get_word_start_escaped_space() {
        todo!("support for escaped whitespace is not yet implemented");

        let buffer = "escaped\\ space";
        let offset = buffer.len();

        let expected = 0;
        let actual = session::get_word_start(buffer, offset);

        assert_eq!(expected, actual);
    }
}

#[cfg(test)]
mod test_get_tab_completion {
    use std::env;
    use std::path::PathBuf;
    use crate::session;

    fn get_resource_path(components: &[&str]) -> PathBuf {
        components.iter().fold(PathBuf::from("tests").join("resources"),
                               |acc, component| acc.join(component))
    }

    /// this method changes the cwd, only run with `--test-threads 1`

    #[ignore]
    #[test]
    fn test_get_tab_completion_empty_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let old_cwd = env::current_dir()?;
        let new_cwd = get_resource_path(&["a_directory"]).canonicalize()?;

        let new_path = get_resource_path(&["some_other_directory"]).canonicalize()?;

        env::set_var("PATH", new_path);

        env::set_current_dir(new_cwd.as_path())?;
        let actual = session::get_tab_completions("", true);
        env::set_current_dir(old_cwd)?;

        let expected: Vec<String> = vec![
            // todo: add trailing slash for directory name (ie "directory/")
            String::from("directory"),

            // form path
            String::from("a_final_file"),
            String::from("yet_another_file"),
        ];

        assert_eq!(expected, actual);

        Ok(())
    }

    #[ignore]
    #[test]
    fn test_get_tab_completion() -> Result<(), Box<dyn std::error::Error>> {
        let old_cwd = env::current_dir()?;
        let new_cwd = get_resource_path(&["a_directory"]).canonicalize()?;

        let new_path = get_resource_path(&["some_other_directory"]).canonicalize()?;

        env::set_var("PATH", new_path);

        env::set_current_dir(new_cwd.as_path())?;
        let actual = session::get_tab_completions("a", true);
        env::set_current_dir(old_cwd)?;

        let expected: Vec<String> = vec![
            // from path
            String::from("a_final_file"),
        ];

        assert_eq!(expected, actual);

        Ok(())
    }

    #[ignore]
    #[test]
    fn test_get_tab_completion_with_parent() -> Result<(), Box<dyn std::error::Error>> {
        let old_cwd = env::current_dir()?;
        let new_cwd = get_resource_path(&["a_directory"]).canonicalize()?;

        let new_path = get_resource_path(&["some_other_directory"]).canonicalize()?;

        env::set_var("PATH", new_path);

        env::set_current_dir(new_cwd.as_path())?;
        let actual = session::get_tab_completions("./", true);
        env::set_current_dir(old_cwd)?;

        let expected: Vec<String> = vec![
            String::from("./a_file"),
            String::from("./directory"),
            String::from("./some_other_file"),
        ];

        assert_eq!(expected, actual);

        Ok(())
    }

    #[ignore]
    #[test]
    fn test_get_tab_completion_non_cmd_empty_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let old_cwd = env::current_dir()?;
        let new_cwd = get_resource_path(&["a_directory"]).canonicalize()?;

        let new_path = get_resource_path(&["some_other_directory"]).canonicalize()?;

        env::set_var("PATH", new_path);

        env::set_current_dir(new_cwd.as_path())?;
        let actual = session::get_tab_completions("", false);
        env::set_current_dir(old_cwd)?;

        let expected: Vec<String> = vec![
            String::from("a_file"),
            String::from("another_file"),
            String::from("directory"),
            String::from("some_other_file"),
        ];

        assert_eq!(expected, actual);

        Ok(())
    }

    #[ignore]
    #[test]
    fn test_get_tab_completion_non_cmd() -> Result<(), Box<dyn std::error::Error>> {
        let old_cwd = env::current_dir()?;
        let new_cwd = get_resource_path(&["a_directory"]).canonicalize()?;

        let new_path = get_resource_path(&["some_other_directory"]).canonicalize()?;

        env::set_var("PATH", new_path);

        env::set_current_dir(new_cwd.as_path())?;
        let actual = session::get_tab_completions("a", false);
        env::set_current_dir(old_cwd)?;

        let expected: Vec<String> = vec![
            String::from("a_file"),
            String::from("another_file"),
        ];

        assert_eq!(expected, actual);

        Ok(())
    }

    #[ignore]
    #[test]
    fn test_get_tab_completion_non_cmd_with_parent() -> Result<(), Box<dyn std::error::Error>> {
        let old_cwd = env::current_dir()?;
        let new_cwd = get_resource_path(&["a_directory"]).canonicalize()?;

        let new_path = get_resource_path(&["some_other_directory"]).canonicalize()?;

        env::set_var("PATH", new_path);

        env::set_current_dir(new_cwd.as_path())?;
        let actual = session::get_tab_completions("directory/", false);
        env::set_current_dir(old_cwd)?;

        let expected: Vec<String> = vec![
            String::from("a_child"),
        ];

        assert_eq!(expected, actual);

        Ok(())
    }
}