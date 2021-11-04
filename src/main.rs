#[macro_use]
extern crate clap;

mod builtins;

use std::env;
use std::io;
use std::io::Write;
use std::process::Command;

use clap::Arg;

fn prompt() -> String {
    format!("[{}] $ ", env::var("USER").unwrap())
}

fn get_input() -> Option<Vec<String>> {
    let mut buffer = String::new();

    if let Err(err) = io::stdin().read_line(&mut buffer) {
        eprintln!("Error readinn from stind: {}", err)
    }

    let argv = shlex::split(buffer.as_str());

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
// todo: more builtins
// todo: allow for standalone or wrapper
fn main() {
    let matches = app_from_crate!()
        .arg(
            Arg::with_name("cmd")
                .help("the base command for the shell to wrap around")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let base = matches.value_of("cmd").unwrap();

    env::set_var("WRASH_BASE", base);
    env::set_var("WRASH_MODE", "wrapped");

    loop {
        print!("{}", prompt());
        let _ = io::stdout().flush();

        let argv = if let Some(a) = get_input() {
            a
        } else {
            continue;
        };

        match argv[0].as_str() {
            "exit" => builtins::exit(&argv),
            "cd" => builtins::cd(&argv),
            "mode" => builtins::mode(&argv), // todo: allow for switching between a "normal" and a wrapped shell
            "setmode" => builtins::setmode(&argv),
            _ => run(base, argv.as_slice()),
        };
    }
}
