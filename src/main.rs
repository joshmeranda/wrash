#[macro_use]
extern crate clap;

#[macro_use]
extern crate serde_derive;

mod builtins;
mod completion;
mod error;
mod history;
mod session;

use std::{env, thread};
use std::io::{self, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::error::WrashError;
use clap::Arg;

use crate::history::History;
use crate::session::{Session, SessionMode};

/// Generate the command prompt
///
/// todo: allow some user configurability
fn prompt() -> String {
    format!("[{}] $ ", env::var("USER").unwrap())
}

fn run(command: &str, args: &[String]) -> Result<(), WrashError> {
    let proc = Command::new(command).args(args).spawn();

    let code = match proc {
        Err(err) => {
            eprintln!("Error starting '{}': {}", command, err);

            -1
        }
        Ok(mut child) => match child.wait() {
            // todo: better handle signal interrupts here (don't just return 255)
            Ok(status) => status.code().unwrap_or(255),
            Err(err) => {
                eprintln!("command '{}' never started: {}", command, err);

                -3
            }
        },
    };

    if code == 0 {
        Ok(())
    } else {
        Err(WrashError::NonZeroExit(code))
    }
}

fn wrapped_main() -> Result<(), WrashError> {
    let matches = app_from_crate!()
        .arg(
            Arg::with_name("cmd")
                .help("the base command for the shell to wrap around")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("is_frozen")
                .help("freeze the session mode to 'wrapped', limiting the user's access to the system to the wrapped command and builtins")
                .short("F")
                .long("--frozen")
        )
        .get_matches();

    let history = match History::new() {
        Ok(history) => history,
        Err(err) => {
            eprintln!("Could not establish proper history: {}\ncontinuing with in memory error (you will not be able to sync history changes", err);
            History::empty()
        }
    };

    let base = matches.value_of("cmd").unwrap();
    let is_frozen = matches.is_present("is_frozen");

    let mut session = Session::new(history, is_frozen, base, SessionMode::Wrapped);

    let mut should_continue = true;
    let mut result = Ok(());

    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();

    ctrlc::set_handler(|| { });

    while should_continue {
        let _ = io::stdout().flush();

        // todo: we will likely want to do the splitting ourselves or add post-processing to allow for globbing so that we can handle globs
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

        result = match argv[0].as_str() {
            "exit" => {
                // todo: differentiate between successful run of exit and failed argument parsing for exit
                should_continue = false;
                builtins::exit(&argv)
            }
            "cd" => builtins::cd(&mut stderr, &argv),
            "mode" => builtins::mode(&mut stdout, &mut stderr, &mut session, &argv),
            "?" => builtins::help(&argv),
            "history" => builtins::history(&mut stdout, &mut stderr, &mut session, &argv),
            _ => match session.mode() {
                SessionMode::Wrapped => run(base, argv.as_slice()),
                SessionMode::Normal => run(argv[0].as_str(), &argv[1..]),
            },
        };

        session.push_to_history(cmd.as_str(), builtins::is_builtin(argv[0].as_str()));
    }

    result
}

fn main() {
    if let Err(err) = wrapped_main() {
        match err {
            WrashError::NonZeroExit(n) => std::process::exit(n),
            WrashError::FailedIo(err) => eprintln!("Error: {}", err),
            WrashError::Custom(s) => println!("Error: {}", s),
        }
    }
}
