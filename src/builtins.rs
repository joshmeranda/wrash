use std::path::PathBuf;
use std::process::Command;

use clap::{Arg, ErrorKind};
use directories::UserDirs;

type BuiltinResult = Result<i32, i32>;

/// handle_matches is designed to allow for clean and uniform argument handling.
macro_rules! handle_matches {
    ($app:ident, $argv:ident) => {
        match $app.get_matches_from_safe($argv) {
            Err(err) => match err.kind {
                ErrorKind::HelpDisplayed => {
                    println!("{}", err);

                    return Ok(0);
                }
                _ => {
                    eprintln!("Error: {}", err);

                    return Err(1);
                }
            },
            Ok(m) => m,
        }
    };
}

/// Exit is a builtin for exiting out of the current shell session.
pub fn exit(argv: &[String]) -> BuiltinResult {
    let app = app_from_crate!().name("exit").arg(
        Arg::with_name("code")
            .help("the number to use for the exit status if supplied")
            .default_value("0")
            .validator(|code| match code.parse::<i32>() {
                Err(err) => Err(format!(
                    "could not parse integer from value '{}': {}",
                    code, err
                )),
                Ok(n) => {
                    if n < 0 {
                        Err(String::from("exit code must not be negative"))
                    } else {
                        Ok(())
                    }
                }
            }),
    );

    let matches = handle_matches!(app, argv);

    let code: i32 = matches.value_of("code").unwrap().parse().unwrap();

    std::process::exit(code as i32);
}

/// CD is builtin for changing the current working directory in the shell.
pub fn cd(argv: &[String]) -> BuiltinResult {
    let app = app_from_crate!().name("cd").arg(
        Arg::with_name("directory")
            .help("the directory to change into")
            .validator(|dir| {
                if !PathBuf::from(dir.as_str()).is_dir() {
                    Err(format!("no such file or directory '{}'", dir))
                } else {
                    Ok(())
                }
            }),
    );

    let matches = handle_matches!(app, argv);

    let target = if matches.is_present("directory") {
        PathBuf::from(matches.value_of("directory").unwrap())
    } else {
        let dirs = match UserDirs::new() {
            Some(dirs) => dirs,
            None => {
                eprintln!("could not determine the home directory for the current user");

                return Err(2);
            }
        };

        dirs.home_dir().to_path_buf()
    };

    if let Err(err) = std::env::set_current_dir(target) {
        eprintln!("Error changing directories: {}", err)
    }

    Ok(0)
}

/// Fall will make the given argument "fall" down to the kernal as a command to be run.
pub fn fall(argv: &[String]) -> BuiltinResult {
    let app = app_from_crate!()
        .name("fall")
        .arg(Arg::with_name("command").multiple(true).required(true));

    let matches = handle_matches!(app, argv);

    // args still contains the initial command (binary / path to executable) which needs to be extracted
    let mut args: Vec<&str> = matches.values_of("command").unwrap().collect();

    let command = args.remove(0);
    let args = args;

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
        Ok(0)
    } else {
        Err(code)
    }
}
