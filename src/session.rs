use std::cmp::Ordering;
use std::io::Write;
use std::{env, io};
use termion::clear::{AfterCursor, All};
use termion::cursor::{Goto, Restore, Right, Save};
use termion::event::Key;
use termion::input::TermRead;

use crate::history::{History, HistoryEntry, HistoryIterator};
use crate::prompt;
use termion::raw::IntoRawMode;

pub struct Session<'shell> {
    history: History,

    base: &'shell str,
}

impl<'shell> Session<'shell> {
    pub fn new(history: History, base: &'shell str) -> Session<'shell> {
        Session { history, base }
    }

    pub fn get_mode(&self) -> Result<String, env::VarError> {
        env::var("WRASH_MODE")
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

        let mut history_offset = None;
        let mut buffer_bak = None;

        let prompt = prompt();

        write!(stdout, "{}{}", Save, prompt)?;
        stdout.flush()?;

        write!(stdout, "{}", Right(1))?;

        for key in stdin.keys() {
            match key.unwrap() {
                Key::Char('\n') => break,
                Key::Char(c) => {
                    if offset == buffer.len() {
                        buffer.push(c);
                        offset += 1;
                    } else {
                        buffer.insert(offset, c);
                        offset += 1;
                    }
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

                // todo: filter to only include history entries from the current mode
                Key::Up => {
                    match history_offset {
                        Some(n) => {
                            if n < self.history.len() {
                                history_offset = Some(n + 1);
                            }
                        }
                        None => {
                            history_offset = Some(0);
                            buffer_bak = Some(buffer.clone());
                        }
                    };

                    if let Some(entry) = self.history.get_from_end(history_offset.unwrap()) {
                        buffer = entry.get_command();
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
                        if let Some(entry) = self.history.get_from_end(history_offset) {
                            buffer = entry.get_command();
                        }
                    }
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

            // todo: will have issues when deleting characters
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
    /// todo: check if the given command is a builtin to avoid adding unneeded base command
    pub fn push_to_history(&mut self, command: &str) {
        match self.get_mode() {
            Ok(m) => {
                let entry = HistoryEntry::new(command.trim().to_string(), if m == "wrapped" { Some(self.get_base()) } else { None }, m);

                self.history.push(entry);
            },
            Err(err) => eprintln!(
                concat!("could not determine the current wrash execution mode: {}\n",
                "Please verify that 'WRASH_MODE' is set to one of the valid options using 'setmode'"), err)
        }
    }

    pub fn history_iter(&self) -> HistoryIterator {
        self.history.iter()
    }

    pub fn history_sync(&self) -> Result<(), std::io::Error> {
        self.history.sync()
    }
}
