#[macro_use]
extern crate clap;

mod builtins;
mod history;

use std::env;
use std::io;
use std::io::{Read, Write};
use std::process::Command;

use clap::Arg;

use termion::clear::{AfterCursor, BeforeCursor};
use termion::cursor::{Left, Restore, Right, Save};
use termion::event::Key;
use termion::input::TermRead;

use termion::raw::IntoRawMode;

use crate::history::{History, HistoryEntry};

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
    ///
    /// todo: set cursor to end of command from history
    /// todo: return resutl to use all the write! results
    pub fn take_input(&mut self) -> String {
        let stdout = io::stdout();
        let mut stdout = stdout.lock().into_raw_mode().unwrap();

        let stdin = io::stdin();
        let mut stdin = stdin.lock();

        let mut buffer = String::new();

        let mut offset = 0usize;
        let mut history_offset = None;
        let mut buffer_bak = None;

        let prompt = prompt();

        write!(stdout, "{}{}", Save, prompt).unwrap();
        stdout.flush().unwrap();

        write!(stdout, "{}", Right(1));

        for key in stdin.keys() {
            match key.unwrap() {
                Key::Char('\n') => {
                    writeln!(stdout, "{}", Restore);
                    stdout.flush();
                    break
                },
                Key::Char(c) => {
                    if offset == buffer.len() {
                        buffer.push(c);
                        offset += 1;
                    } else {
                        buffer.insert(offset, c);
                        offset += 1;
                    }

                },
                Key::Backspace => {
                    if offset != 0 && offset == buffer.len() {
                        buffer.pop();
                        offset -= 1;
                    } else if offset != 0 {
                        buffer.remove(offset);
                    }
                }
                Key::Left => {
                    if offset != 0 {
                        offset -= 1;
                    }
                },
                Key::Right => {
                    if offset < buffer.len() {
                        offset += 1;
                    }
                },
                Key::Up => {
                    match history_offset {
                        Some(n) => if n < self.history.len() {
                            history_offset = Some(n + 1);
                        },
                        None => {
                            history_offset = Some(0);
                            buffer_bak = Some(buffer.clone());
                        }
                    };

                    if let Some(entry) = self.history.get_from_end(history_offset.unwrap()) {
                        buffer = entry.get_command();
                    }
                },
                Key::Down => {
                    if let Some(n) = history_offset {
                        if n > 0 {
                            history_offset = Some(n - 1);
                        } else if n == 0 {
                            history_offset = None;

                            buffer = buffer_bak.unwrap();
                            buffer_bak = None;
                        }
                    }

                    if let Some(history_offset) = history_offset {
                        if let Some(entry) = self.history.get_from_end(history_offset) {
                            buffer = entry.get_command();
                        }
                    }
                }
                _ => { /* do nothing */ }
            };

            // todo: will have issues when deleting characters
            write!(stdout, "{}{}{}{}{}{}", Restore, AfterCursor, prompt, buffer, Restore, Right((prompt.len() + offset) as u16));
            stdout.flush();
        }

        buffer
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
}

/// Generate the command prompt
///
/// todo: allow some user configurability
fn prompt() -> String {
    format!("[{}] $ ", env::var("USER").unwrap())
}

fn run(command: &str, args: &[String]) -> Result<i32, i32> {
    let proc = Command::new(command).args(args).spawn();

    let code = match proc {
        Err(err) => {
            eprintln!("Error starting '{}': {}", command, err);

            -1
        }
        Ok(mut child) => match child.wait() {
            // todo: better handle signal interrupts here (don't just return -1)
            Ok(status) => status.code().unwrap_or(-2),
            Err(err) => {
                eprintln!("command '{}' never started: {}", command, err);

                -3
            }
        },
    };

    if code == 0 {
        Ok(code)
    } else {
        Err(code)
    }
}

// todo: history
// todo: up-arrow for last command(s)
// todo: shell mode enum
//   would remove the shell's dependency on the WRASH_MODE environment variable
// todo: shell session?
fn main() {
    let matches = app_from_crate!()
        .arg(
            Arg::with_name("cmd")
                .help("the base command for the shell to wrap around")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let history = History::new();
    let base = matches.value_of("cmd").unwrap();

    let mut session = Session::new(history, base);

    env::set_var("WRASH_BASE", base); // todo: not needed
    env::set_var("WRASH_MODE", "wrapped");

    loop {
        let _ = io::stdout().flush();

        // todo: we will likely want to do the splitting ourselves or add post-processing to allow for globbing so that we can handle globs better
        let cmd = session.take_input();
        let argv = match shlex::split(cmd.as_str()) {
            Some(args) => args,
            None => {
                eprintln!("Error splitting command line arguments");
                continue;
            }
        };

        if argv.len() == 0 {
            continue
        }

        let _result = match argv[0].as_str() {
            "exit" => builtins::exit(&argv),
            "cd" => builtins::cd(&argv),
            "mode" => builtins::mode(&argv),
            "setmode" => builtins::setmode(&argv),
            "help" => builtins::help(&argv),
            "history" => builtins::history(&mut session, &argv),
            _ => match session.get_mode() {
                Ok(m) => match m.as_str() {
                    "wrapped" => run(base, argv.as_slice()),
                    "normal" => run(argv[0].as_str(), &argv[1..]),
                    _ => unreachable!(),
                },
                Err(err) => {
                    eprintln!(
                        concat!("could not determine the current wrash execution mode: {}\n",
                        "Please verify that 'WRASH_MODE' is set to one of the valid options using 'setmode'"), err);

                    Err(-1)
                }
            },
        };

        session.push_to_history(cmd.as_str());
    }

    // todo: consider writing to temporary file to be merged into the master history later on error
    // if let Err(err) = history.sync() {
    //     eprintln!(
    //         "Error: could not write sessions history to history file: {}",
    //         err
    //     );
    // }
}
