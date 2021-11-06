#[macro_use]
extern crate clap;

mod builtins;
mod history;

use std::env;
use std::io;
use std::io::Write;
use std::process::Command;

use clap::Arg;
use crate::history::{History, HistoryEntry};

/// Generate the command prompt
///
/// todo: allow some user configurability
fn prompt() -> String {
    format!("[{}] $ ", env::var("USER").unwrap())
}

// todo: a shell sessions would simplify this  function signature
fn get_input<'base: 'history, 'history>(history: &'history mut History<'base>, base: &'base str, mode: String) -> Option<Vec<String>> {
    let mut buffer = String::new();

    if let Err(err) = io::stdin().read_line(&mut buffer) {
        eprintln!("Error reading from stind: {}", err)
    }

    let argv = shlex::split(buffer.as_str());

    let entry = HistoryEntry::new(buffer, base, mode);
    history.push(entry);

    match argv {
        Some(argv) => {
            if argv.is_empty() {
                None
            } else {
                Some(argv)
            }
        }
        None => None,
    }
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

    let mut history = History::new();

    let base = matches.value_of("cmd").unwrap();

    env::set_var("WRASH_BASE", base);
    env::set_var("WRASH_MODE", "wrapped");

    loop {
        print!("{}", prompt());
        let _ = io::stdout().flush();

        let argv = if let Some(a) = get_input(&mut history, base, env::var("WRASH_MODE").unwrap().to_string()) {
            a
        } else {
            continue;
        };

        match argv[0].as_str() {
            "exit" => builtins::exit(&argv),
            "cd" => builtins::cd(&argv),
            "mode" => builtins::mode(&argv), // todo: allow for switching between a "normal" and a wrapped shell
            "setmode" => builtins::setmode(&argv),
            "help" => builtins::help(&argv),
            _ => run(base, argv.as_slice()),
        };
    }

    // todo: consider writing to temporary file to be merged into the master history later
    if let Err(err) = history.sync() {
        eprintln!("Error: could not write sessions history to history file: {}", err);
    }
}
