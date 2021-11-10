#[macro_use]
extern crate clap;

mod builtins;
mod history;
mod session;

use std::env;
use std::io::{self, Write};
use std::process::Command;

use clap::Arg;

use crate::history::History;
use crate::session::{Session, SessionMode};

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

    let mut session = Session::new(history, base, SessionMode::Wrapped);

    loop {
        let _ = io::stdout().flush();

        // todo: we will likely want to do the splitting ourselves or add post-processing to allow for globbing so that we can handle globs better
        let cmd = match session.take_input() {
            Ok(c) => c,
            Err(err) => {
                eprintln!("Error: could not take input: {}", err);
                continue;
            }
        };

        let argv = match shlex::split(cmd.as_str()) {
            Some(args) => args,
            None => {
                eprintln!("Error splitting command line arguments");
                continue;
            }
        };

        if argv.is_empty() {
            continue;
        }

        let _result = match argv[0].as_str() {
            "exit" => builtins::exit(&argv),
            "cd" => builtins::cd(&argv),
            "mode" => builtins::mode(&session, &argv),
            "setmode" => builtins::setmode(&mut session, &argv),
            "help" => builtins::help(&argv),
            "history" => builtins::history(&mut session, &argv),
            _ => match session.get_mode() {
                SessionMode::Wrapped => run(base, argv.as_slice()),
                SessionMode::Normal => run(argv[0].as_str(), &argv[1..]),
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
