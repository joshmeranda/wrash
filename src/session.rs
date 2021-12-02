use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::io;
use std::io::Write;
use std::str::FromStr;

use termion::clear::{AfterCursor, All};
use termion::cursor::{Goto, Restore, Right, Save};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use crate::history::{History, HistoryEntry, HistoryIterator};

use crate::prompt;

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
        let mut history_offset = None;
        let mut buffer_bak = None;

        let prompt = prompt();

        write!(stdout, "{}{}", Save, prompt)?;
        stdout.flush()?;

        write!(stdout, "{}", Right(1))?;

        // todo: do not append the base command when in wrapped mode
        for key in stdin.keys() {
            match key.unwrap() {
                Key::Char('\n') => break,
                Key::Char(c) => {
                    if offset == buffer.len() {
                        buffer.push(c);
                    } else {
                        buffer.insert(offset, c);
                    }

                    offset += 1;
                }
                Key::Backspace => {
                    if offset > 0 {
                        buffer.remove(offset - 1);
                        offset -= 1;
                    }
                }
                Key::Delete => {
                    if offset < buffer.len() {
                        buffer.remove(offset);
                        offset -= 1;
                    }
                }
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
                        buffer = entry.get_command();
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
                            buffer = entry.get_command();
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
                Key::Ctrl('l') => write!(
                    stdout,
                    "{}{}{}{}{}",
                    Restore,
                    All,
                    Right(offset as u16),
                    Goto(1, 1),
                    Save
                )?,

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
