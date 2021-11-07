#[macro_use]
extern crate clap;

mod builtins;
mod history;

use std::env;
use std::io;
use std::io::Write;
use std::process::Command;

use crate::history::{History, HistoryEntry};
use clap::Arg;

pub struct Session<'shell> {
    history: History,

    base: &'shell str,
}

impl<'shell> Session<'shell> {
    pub fn new(history: History, base: &'shell str) -> Session<'shell> {
        Session { history, base }
    }

    pub fn mode(&self) -> Result<String, env::VarError> {
        env::var("WRASH_MODE")
    }

    pub fn take_input(&mut self) -> String {
        let mut buffer = String::new();

        if let Err(err) = io::stdin().read_line(&mut buffer) {
            eprintln!("Error reading from stind: {}", err)
        }

        buffer
    }

    pub fn push_to_history(&mut self, command: &str) {
        match self.mode() {
            Ok(m) => {
                let entry = HistoryEntry::new(command.trim().to_string(), None, m);

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

    env::set_var("WRASH_BASE", base);
    env::set_var("WRASH_MODE", "wrapped");

    loop {
        print!("{}", prompt());
        let _ = io::stdout().flush();

        let cmd = session.take_input();
        let argv = match shlex::split(cmd.as_str()) {
            Some(args) => args,
            None => {
                eprintln!("Error splitting command line arguments");
                continue;
            }
        };

        let _result = match argv[0].as_str() {
            "exit" => builtins::exit(&argv),
            "cd" => builtins::cd(&argv),
            "mode" => builtins::mode(&argv), // todo: allow for switching between a "normal" and a wrapped shell
            "setmode" => builtins::setmode(&argv),
            "help" => builtins::help(&argv),
            "history" => builtins::history(&mut session, &argv),
            _ => match session.mode() {
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

    // todo: consider writing to temporary file to be merged into the master history later
    // if let Err(err) = history.sync() {
    //     eprintln!(
    //         "Error: could not write sessions history to history file: {}",
    //         err
    //     );
    // }
}
