// todo: all documentation could be cleaned up.

#[macro_use]
extern crate clap;

#[macro_use]
extern crate serde_derive;

mod argv;
mod builtins;
mod completion;
mod error;
mod history;
mod session;

use std::env;
use std::io::{self, Write};
use std::process::Command;

use crate::argv::expand;
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

fn main() {
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

    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();

    if let Err(err) = ctrlc::set_handler(|| {}) {
        eprintln!("Error: {}", err);
        return;
    }

    loop {
        let _ = io::stdout().flush();

        let cmd = match session.take_input() {
            Ok(c) => c,
            Err(err) => {
                eprintln!("Error: could not take input: {}", err);
                continue;
            }
        };

        let trimmed = cmd.trim();

        if trimmed.is_empty() {
            continue;
        }

        let argv = match expand::expand(trimmed) {
            Ok(argv) => argv,
            Err(err) => {
                eprintln!("Error expanding command line arguments: {}", err);
                session.push_to_history(trimmed, builtins::is_builtin(trimmed.split_whitespace().next().unwrap()));
                continue;
            }
        };

        let result = match argv[0].as_str() {
            // if exit is successful the current process will be exited
            "exit" => {
                if let Err(err) = session.history_sync() {
                    eprintln!(
                        "Error: could not synchronize session history to file system: {}",
                        err
                    );
                }

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

        session.push_to_history(trimmed, builtins::is_builtin(argv[0].as_str()));

        if let Err(err) = result {
            match err {
                WrashError::NonZeroExit(_) => {}
                WrashError::FailedIo(err) => eprintln!("Error: {}", err),
                WrashError::Custom(s) => println!("Error: {}", s),
            }
        }
    }
}
