use std::env;
use std::path::PathBuf;

use crate::error::StatusError;
use crate::Session;
use clap::{Arg, ErrorKind, SubCommand};
use directories::UserDirs;

type BuiltinResult = Result<(), StatusError>;

// todo: add tests for these builtins

/// handle_matches is designed to allow for clean and uniform argument handling.
macro_rules! handle_matches {
    ($app:ident, $argv:ident) => {
        match $app.get_matches_from_safe($argv) {
            Err(err) => match err.kind {
                ErrorKind::HelpDisplayed => {
                    println!("{}", err);

                    return Ok(());
                }
                _ => {
                    eprintln!("Error: {}", err);

                    return Err(StatusError { code: 1 });
                }
            },
            Ok(m) => m,
        }
    };
}

/// Check if a command is a builtin or not.
pub fn is_builtin(command: &str) -> bool {
    matches!(
        command,
        "exit" | "cd" | "mode" | "setmode" | "help" | "history"
    )
}

/// Exit is a builtin for exiting out of the current shell session.
pub fn exit(argv: &[String]) -> BuiltinResult {
    let app = app_from_crate!()
        .name("exit")
        .about("exit the shell with the given status code")
        .arg(
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

    if code == 0 {
        Ok(())
    } else {
        Err(StatusError { code })
    }
}

/// CD is builtin for changing the current working directory in the shell.
pub fn cd(argv: &[String]) -> BuiltinResult {
    let app = app_from_crate!()
        .name("cd")
        .about("change the current working directory")
        .arg(
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

                return Err(StatusError { code: 2 });
            }
        };

        dirs.home_dir().to_path_buf()
    };

    if let Err(err) = std::env::set_current_dir(target) {
        eprintln!("Error changing directories: {}", err)
    }

    Ok(())
}

/// Print the status of the current node.
///
/// todo: consider merging `setmode` and `mode` into one (ie `mode [MODE]`) and
///       print the current mode if no argument is given
pub fn mode(session: &Session, argv: &[String]) -> BuiltinResult {
    let app = app_from_crate!()
        .name("mode")
        .about("get the current wrash execution mode");

    let _matches = handle_matches!(app, argv);

    println!("{}", session.mode());

    Ok(())
}

/// Set the current shell mode.
pub fn setmode(session: &mut Session, argv: &[String]) -> BuiltinResult {
    let app = app_from_crate!()
        .name("mode")
        .about("set the wrash execution mode")
        .arg(
            Arg::with_name("mode")
                .help("the mode to set the shell to")
                .possible_values(&["wrapped", "normal"])
                .required(true),
        );

    let matches = handle_matches!(app, argv);

    let new_mode = matches.value_of("mode").unwrap().parse().unwrap();

    if session.set_mode(new_mode).is_err() {
        eprintln!("Error: could not set session mode, session is frozen");
        Err(StatusError { code: 1 })
    } else {
        Ok(())
    }
}

/// Show help text for using the shell.
pub fn help(argv: &[String]) -> BuiltinResult {
    let app = app_from_crate!()
        .name("help")
        .about("show some basic information about WraSh and how to use it");

    handle_matches!(app, argv);

    println!(
        r"Thanks for using WraSh!

WraSh is designed to provide a very minimal 'no frills' interactive wrapper
shell around a base command. For example if the base command was 'git', you
could call 'add -A' rather then 'git add -A'.

You may also call all the normal commands on your system with WraSh. You need
to simply change the operation mode with 'setmode normal' run any commands you
want like 'whoami' then change back to wrapper mode 'setmode wrapper'

Below is a list of supported builtins, pass '--help' to any o them for more information:
    exit
    cd
    mode
    setmode
    help"
    );

    Ok(())
}

/// Examine and manipulate the command history, if the command was run in "wrapped" mode,
///
/// todo: show / search commands (allow specifying offset or number)
/// todo: allow filtering commands with regex
pub fn history(session: &mut Session, argv: &[String]) -> BuiltinResult {
    let app = app_from_crate!()
        .name("history")
        .max_term_width(80)
        .about("examine and manipulate the command history, if session is frozen this command wil ALWAYS fail")
        .after_help("if no subcommand is specified, then only commands run with the same mode and base command  along with builtins are shown")
        .subcommand(
            SubCommand::with_name("sync")
                .about("flush the current in-memory history into the history file"),
        )
        .subcommand(SubCommand::with_name("filter").about("filter history to only show the command you want to see")
            .arg(Arg::with_name("filter-mode").short("m").long("mode").min_values(0).help("only show commands from the given shell execution mode, if no value is given the current execution mode is used"))
            .arg(Arg::with_name("filter-base").short("b").long("base").min_values(0).help("only show commands whose 'base' matches the given base or have no base, if no value is given the current value is used"))
        );

    let matches = handle_matches!(app, argv);

    match matches.subcommand() {
        ("sync", Some(_)) => {
            if let Err(err) = session.history_sync() {
                eprintln!("Error saving to history file: {}", err)
            }
        }
        ("filter", Some(sub_matches)) => {
            let filter_base = sub_matches.is_present("filter-base");
            let filter_mode = sub_matches.is_present("filter-mode");

            let entries = session.history_iter().filter(|entry| {
                if filter_mode && entry.mode != session.mode() {
                    return false;
                }

                if entry.base.is_some()
                    && filter_base
                    && entry.base.as_ref().unwrap().as_str() != session.base
                {
                    return false;
                }

                true
            });

            for (i, entry) in entries.enumerate() {
                println!("{}: {}", i, entry.get_command());
            }
        }
        _ => {
            for (i, entry) in session
                .history_iter()
                .filter(|entry| {
                    entry.is_builtin
                        || (entry.mode == session.mode()
                            && (entry.base.is_none()
                                || entry.base.as_ref().unwrap() == session.base))
                })
                .enumerate()
            {
                println!("{}: {}", i, entry.get_command());
            }
        }
    }

    Ok(())
}
