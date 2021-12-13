use std::cmp::{max, Ordering};
use std::env;
use std::fmt::{Display, Formatter};
use std::io::{self, Write};
use std::path::{Component, Path};
use std::str::FromStr;

use termion::clear::{AfterCursor, All};
use termion::cursor::{Goto, Right};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use faccess::PathExt;

use crate::completion;
use crate::history::{History, HistoryEntry, HistoryIterator};

use crate::prompt;

/// Get the position in a string at which the current word begins.
fn get_previous_boundary(buffer: &str, cursor_offset: usize) -> usize {
    if cursor_offset == 0 {
        return 0;
    }

    let mut chars = buffer.chars().rev();

    let initial_is_boundary = chars.nth(buffer.len() - cursor_offset).unwrap() == ' ';

    let mut position = cursor_offset - 1;

    for c in chars {
        if !initial_is_boundary && c == ' ' || initial_is_boundary && c != ' ' {
            break;
        }

        position -= 1;
    }

    position
}

/// Get the position of the next word.
fn get_next_boundary(buffer: &str, cursor_offset: usize) -> usize {
    if cursor_offset == buffer.len() {
        return buffer.len();
    }

    let mut chars = buffer.chars();

    let initial_is_boundary = chars.nth(cursor_offset).unwrap() == ' ';

    let mut position = cursor_offset + 1;

    for c in chars {
        if !initial_is_boundary && c == ' ' || initial_is_boundary && c != ' ' {
            break;
        }

        position += 1;
    }

    position
}

/// Get the tab completion values.
///
/// todo: ignore non-unicode strings
fn get_tab_completions(prefix: &str, is_command: bool) -> Vec<String> {
    let prefix_path = Path::new(prefix);

    let has_parent = if let Some(parent) = prefix_path.parent() {
        !parent.as_os_str().is_empty()
    } else {
        false
    };
    let has_cur_dir = Some(Component::CurDir) == prefix_path.components().next();

    let in_dir = completion::search_prefix(prefix_path).unwrap();

    if is_command {
        // if the prefix has a parent component, search for directories or executables
        if has_parent {
            return in_dir
                .filter_map(|path| {
                    if path.executable() {
                        Some(path.to_string_lossy().to_string())
                    } else {
                        None
                    }
                })
                .collect();
        }

        // if the prefix does not have a parent component, search on path or directories
        let path_var = env::var("PATH").unwrap_or_else(|_| "".to_string());
        let in_path = completion::search_path(prefix_path, path_var.as_str())
            .unwrap()
            .filter_map(|path| {
                if !has_cur_dir {
                    Some(path.to_string_lossy().to_string())
                } else {
                    None
                }
            });

        in_dir
            .filter_map(|path| {
                if path.is_dir() || path.executable() && has_cur_dir {
                    Some(path.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .chain(in_path)
            .collect()
    } else {
        in_dir
            .map(|path| path.to_string_lossy().to_string())
            .collect()
    }
}

/// Get a common prefix found in all string in values.dd
///
/// todo: consider using a binary search rather than iteratively popping characters off the end
fn get_common_prefix<S: AsRef<str> + Display>(values: &[S]) -> Option<String> {
    if values.as_ref().is_empty() {
        return None;
    }

    // use value with the shortest length as the first value
    let prefix = values.iter().skip(1).fold(values[0].to_string(), |acc, s| {
        if s.as_ref().len() < acc.len() {
            s.to_string()
        } else {
            acc
        }
    });
    let mut prefix_len = prefix.len();

    let mut is_common = values
        .iter()
        .all(|s| s.as_ref().starts_with(prefix.as_str()));

    while !is_common && prefix_len > 0 {
        is_common = values
            .iter()
            .all(|s| s.as_ref().starts_with(&prefix[..prefix_len]));
        prefix_len -= 1;
    }

    if prefix_len == 0 {
        None
    } else {
        Some(prefix)
    }
}

/// Get how many entries with the given length can fit on a line with
/// `padding_length` spaces between them.
fn get_entries_per_line(padding_length: usize, entry_length: usize, line_length: usize) -> usize {
    // the length of a line (L) is at least equal to the length of each entity
    // (k) times the amount of entities (n) plus the length of padding (m) time
    // the amount of entities - 1:
    //                 L ≥ nk + m(n - 1) -> n ≥ (L + m) / (k + m)
    (line_length + padding_length) / (entry_length + padding_length)
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

pub struct Session<'shell> {
    history: History,

    pub is_frozen: bool,

    pub base: &'shell str,

    mode: SessionMode,
}

impl<'shell> Session<'shell> {
    pub fn new(
        history: History,
        is_frozen: bool,
        base: &'shell str,
        mode: SessionMode,
    ) -> Session<'shell> {
        Session {
            history,
            is_frozen,
            base,
            mode,
        }
    }

    pub fn mode(&self) -> SessionMode {
        self.mode
    }

    // todo: return type is very non_descriptive
    pub fn set_mode(&mut self, mode: SessionMode) -> Result<(), ()> {
        if self.is_frozen {
            Err(())
        } else {
            self.mode = mode;
            Ok(())
        }
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

        let mut was_tab_previous_key = false;

        let prompt = prompt();

        write!(stdout, "{}", prompt)?;
        stdout.flush()?;

        // todo: implement some tab-completion (even if its just files)
        // todo: add support for ctrl+d && ctrl+c
        for key in stdin.keys().filter_map(Result::ok) {
            match key {
                // character deletion
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

                Key::Ctrl('w') => {
                    let word_start = get_previous_boundary(buffer.as_str(), offset);
                    buffer.replace_range(word_start..offset, "");
                    offset = word_start;
                }

                // cursor control
                Key::Ctrl('a') => offset = 0,
                Key::Ctrl('e') => offset = buffer.len(),

                // todo: change to ctrl+left && ctrl+right
                Key::Ctrl('b') => offset = get_previous_boundary(&buffer, offset),
                Key::Ctrl('f') => offset = get_next_boundary(&buffer, offset),

                // screen control
                // todo: write lines and scroll rather than clearing screen
                Key::Ctrl('l') => {
                    write!(stdout, "\r{}{}{}", All, Right(offset as u16), Goto(1, 1),)?
                }

                // exit shell
                Key::Ctrl('d') => {
                    buffer = "exit".to_string();
                    break;
                }

                // tab completion
                Key::Char('\t') => {
                    let word_start = get_previous_boundary(buffer.as_str(), offset);
                    let is_command = word_start == 0;
                    let completions = get_tab_completions(&buffer[word_start..offset], is_command);

                    match completions.len().cmp(&1) {
                        Ordering::Less => { /* do nothing */ }
                        Ordering::Equal => {
                            buffer.replace_range(word_start..offset, completions[0].as_str());
                            offset = buffer.len();
                        }
                        Ordering::Greater => {
                            if was_tab_previous_key {
                                // handle previous tab hit
                                let max_width =
                                    completions.iter().fold(0, |acc, i| max(acc, i.len()));
                                let entries_pre_line = get_entries_per_line(
                                    2,
                                    max_width,
                                    termion::terminal_size().unwrap().0 as usize,
                                );

                                for (i, c) in completions.iter().enumerate() {
                                    if i % entries_pre_line == 0 {
                                        write!(stdout, "\n\r{:<width$}", c, width = max_width)?;
                                    } else {
                                        write!(stdout, "{:<width$}", c, width = max_width + 2)?;
                                    }
                                }
                            } else if let Some(common_prefix) =
                                get_common_prefix(completions.as_slice())
                            {
                                buffer.replace_range(0..offset, common_prefix.as_str());
                                offset = buffer.len();
                            }
                        }
                    }
                }

                Key::Char('\n') => {
                    writeln!(stdout, "\r")?;
                    break;
                }
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

            // todo: replace final carriage return + Right(...) with Left(...)
            write!(
                stdout,
                "\r{}{}{}\r{}",
                AfterCursor,
                prompt,
                buffer,
                Right((prompt.len() + offset) as u16),
            )?;

            stdout.flush()?;

            was_tab_previous_key = key == Key::Char('\t');
        }

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
mod tests {
    mod test_get_next_boundary {
        use crate::session;

        #[test]
        fn get_next_boundary_single_from_start() {
            let buffer = "word";
            let offset = 0;

            let expected = buffer.len();
            let actual = session::get_next_boundary(buffer, offset);

            assert_eq!(expected, actual);
        }

        #[test]
        fn get_next_boundary_multiple_from_start() {
            let buffer = "another word";
            let offset = 0;

            let expected = 7;
            let actual = session::get_next_boundary(buffer, offset);

            assert_eq!(expected, actual);
        }

        #[test]
        fn get_next_boundary_multiple_from_middle() {
            let buffer = "another word";
            let offset = 3;

            let expected = 7;
            let actual = session::get_next_boundary(buffer, offset);

            assert_eq!(expected, actual);
        }

        #[test]
        fn get_next_boundary_multiple_from_end() {
            let buffer = "another word";
            let offset = 7;

            let expected = 8;
            let actual = session::get_next_boundary(buffer, offset);

            assert_eq!(expected, actual);
        }
    }

    mod test_get_previous_boundary {
        use crate::session;

        #[test]
        fn get_previous_boundary_single_from_end() {
            let buffer = "word";
            let offset = buffer.len();

            let expected = 0;
            let actual = session::get_previous_boundary(buffer, offset);

            assert_eq!(expected, actual);
        }

        #[test]
        fn get_previous_boundary_single_from_middle() {
            let buffer = "word";
            let offset = buffer.len() / 2;

            let expected = 0;
            let actual = session::get_previous_boundary(buffer, offset);

            assert_eq!(expected, actual);
        }

        #[test]
        fn get_previous_boundary_single_from_starr() {
            let buffer = "word";
            let offset = 0;

            let expected = 0;
            let actual = session::get_previous_boundary(buffer, offset);

            assert_eq!(expected, actual);
        }

        #[test]
        fn get_previous_boundary_multiple_last_word() {
            let buffer = "some example words";
            let offset = buffer.len();

            let expected = 13;
            let actual = session::get_previous_boundary(buffer, offset);

            assert_eq!(expected, actual);
        }

        #[test]
        fn get_previous_boundary_from_next_word_start() {
            let buffer = "some example";
            let offset = 5;

            let expected = 4;
            let actual = session::get_previous_boundary(buffer, offset);

            assert_eq!(expected, actual);
        }
    }

    /// these methods changes the cwd, only run with `--test-threads 1`
    mod test_get_tab_completion {
        use crate::session;
        use std::env;
        use std::path::{Path, PathBuf};

        fn get_resource_path(components: &[&str]) -> PathBuf {
            vec!["tests", "resources"]
                .iter()
                .chain(components.iter())
                .collect()
        }

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
        fn test_get_tab_completion_with_dot_parent() -> Result<(), Box<dyn std::error::Error>> {
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
        fn test_get_tab_completion_with_dot_dot_parent() -> Result<(), Box<dyn std::error::Error>> {
            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory", "directory"]).canonicalize()?;

            let new_path = "";

            env::set_var("PATH", new_path);

            env::set_current_dir(new_cwd.as_path())?;
            let actual = session::get_tab_completions("../a", true);
            env::set_current_dir(old_cwd.as_path())?;

            let expected: Vec<String> =
                vec![Path::new("..").join("a_file").to_string_lossy().to_string()];

            assert_eq!(expected, actual);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_get_tab_completion_non_cmd_empty_prefix() -> Result<(), Box<dyn std::error::Error>>
        {
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

            let expected: Vec<String> = vec![String::from("a_file"), String::from("another_file")];

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

            let expected: Vec<String> = vec![Path::new("directory")
                .join("a_child")
                .to_string_lossy()
                .to_string()];

            assert_eq!(expected, actual);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_get_tab_completion_non_cmd_with_dot_parent(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]).canonicalize()?;

            let new_path = "";

            env::set_var("PATH", new_path);

            env::set_current_dir(new_cwd.as_path())?;
            let actual = session::get_tab_completions("./", false);
            env::set_current_dir(old_cwd)?;

            let expected: Vec<String> = vec![
                Path::new(".").join("a_file").to_string_lossy().to_string(),
                Path::new(".")
                    .join("another_file")
                    .to_string_lossy()
                    .to_string(),
                Path::new(".")
                    .join("directory")
                    .to_string_lossy()
                    .to_string(),
                Path::new(".")
                    .join("some_other_file")
                    .to_string_lossy()
                    .to_string(),
            ];

            assert_eq!(expected, actual);

            Ok(())
        }
    }

    mod test_common_prefix {
        use crate::session;

        #[test]
        fn test_get_common_prefix_empty_iterator() {
            let values: &[&str] = &[];
            let actual = session::get_common_prefix(values);
            let expected = None;

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_get_common_prefix_no_common_prefix() {
            let actual = session::get_common_prefix(&[
                "some_file_name",
                "another_file_name",
                "i_am_a_directory",
            ]);
            let expected = None;

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_common_prefix_with_common_prefix() {
            let actual = session::get_common_prefix(&["a_file", "a_file_too", "a_file_as_well"]);
            let expected = Some("a_file".to_string());

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_common_prefix_with_partial_common_prefix() {
            let actual = session::get_common_prefix(&[
                "a_file",
                "a_file_too",
                "a_file_as_well",
                "some_new_file",
            ]);
            let expected = None;

            assert_eq!(expected, actual);
        }
    }

    mod test_session {
        use crate::{History, Session, SessionMode};

        #[test]
        fn err_on_set_frozen_session() -> Result<(), Box<dyn std::error::Error>> {
            // todo: allow for clean / empty history for this test to pass reliably
            let mut session = Session::new(
                History::new()?,
                true,
                "nonsense_command",
                SessionMode::Wrapped,
            );

            assert!(session.set_mode(SessionMode::Normal).is_err());

            Ok(())
        }
    }

    mod test_get_entries_per_line {
        use crate::session;

        #[test]
        fn test_entries_per_line_exact_fit() {
            let actual = session::get_entries_per_line(2, 4, 10);
            let expected = 2;

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_entries_per_line_not_enough_for_one() {
            let actual = session::get_entries_per_line(2, 4, 1);
            let expected = 0;

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_entries_per_line_too_much_room() {
            let actual = session::get_entries_per_line(2, 4, 13);
            let expected = 2;

            assert_eq!(expected, actual);
        }
    }
}
