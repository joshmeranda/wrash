#[macro_use]
extern crate clap;

mod builtins;

use std::env;
use std::io;

use std::io::Write;

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

// todo: history
// todo: up-arrow for last command(s)
// todo: more builtins
// todo: allow for standalone or wrapper
fn main() {
    let _matches = app_from_crate!().get_matches();

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
            _ => {
                // todo: construct and run argv through subprocess
                Ok(0)
            }
        };
    }
}
