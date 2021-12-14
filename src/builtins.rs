use std::env;
use std::io::Write;
use std::path::PathBuf;

use crate::error::StatusError;
use crate::Session;
use clap::{Arg, ErrorKind, SubCommand};
use directories::UserDirs;

type BuiltinResult = Result<(), StatusError>;

// todo: make a builtin or runnable type?

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
    matches!(command, "exit" | "cd" | "mode" | "help" | "history")
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
pub fn mode(
    out_writer: &mut impl Write,
    err_writer: &mut impl Write,
    session: &mut Session,
    argv: &[String],
) -> BuiltinResult {
    let app = app_from_crate!()
        .name("mode")
        .about("get or set the current wrash execution mode")
        .arg(
            Arg::with_name("mode")
                .help("if present, the mode to set the shell to")
                .possible_values(&["wrapped", "normal"]),
        );

    let matches = handle_matches!(app, argv);

    if matches.is_present("mode") {
        let new_mode = matches.value_of("mode").unwrap().parse().unwrap();

        if session.set_mode(new_mode).is_err() {
            write!(err_writer, "Error: could not set session mode, session is frozen");
            Err(StatusError { code: 1 })
        } else {
            Ok(())
        }
    } else {
        write!(out_writer, "{}\n", session.mode());

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

WraSh is designed to provide a very minimal interactive wrapper shell around a
base command. For example if the base command was 'git', you could call
'add -A' rather then 'git add -A'.

You may also call all the normal commands on your system with WraSh. You need
to simply change the operation mode with 'mode normal' run any commands you
want like 'whoami' or even 'rm -rf --no-preserve-root /' then change back to
wrapper mode 'setmode wrapper'

Below is a list of supported builtins, pass '--help' to any o them for more information:
    exit       exit the shell with a given status code
    cd         change the current working directory of the shell
    mode       set or modify the current shell execution mode
    ?          show this help text
    history    show and filter shell command history"
    );

    Ok(())
}

/// Examine and manipulate the command history, if the command was run in "wrapped" mode,
///
/// todo: show / search commands (allow specifying offset or number)
/// todo: allow filtering commands with regex
/// todo: fix filtering on base and on mode (very broken not consistent), it should filter based on the given mode and base
/// todo: add --builtin && --no-builtin
pub fn history(
    out_writer: &mut impl Write,
    err_writer: &mut impl Write,
    session: &mut Session,
    argv: &[String],
) -> BuiltinResult {
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
            .arg(Arg::with_name("filter-mode").short("m").long("mode").takes_value(true).help("only show commands from the given shell execution mode, if no value is given the current execution mode is used"))
            .arg(Arg::with_name("filter-base").short("b").long("base").takes_value(true).help("only show commands whose 'base' matches the given base or have no base, if no value is given the current value is used"))
        );

    let matches = handle_matches!(app, argv);

    match matches.subcommand() {
        ("sync", Some(_)) => {
            if let Err(err) = session.history_sync() {
                write!(err_writer, "Error saving to history file: {}", err);
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
                write!(out_writer, "{}: {}\n", i, entry.get_command());
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
                write!(out_writer, "{}: {}\n", i, entry.get_command());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    mod test_exit {
        use crate::builtins;
        use crate::error::StatusError;

        #[test]
        fn test_exit_no_arg() {
            let expected = Ok(());
            let actual = builtins::exit(&["exit".to_string()]);

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_exit_zero() {
            let expected = Ok(());
            let actual = builtins::exit(&["0".to_string()]);

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_exit_1() {
            let expected = Err(StatusError { code: 1 });
            let actual = builtins::exit(&["exit".to_string(), "1".to_string()]);

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_exit_neg_1() {
            let expected = Err(StatusError { code: 1 });
            let actual = builtins::exit(&["exit".to_string(), "-1".to_string()]);

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_exit_non_number() {
            let expected = Err(StatusError { code: 1 });
            let actual = builtins::exit(&["exit".to_string(), "nan".to_string()]);

            assert_eq!(expected, actual);
        }
    }

    mod test_cd {
        use crate::builtins;
        use crate::error::StatusError;
        use directories::UserDirs;
        use std::env;
        use std::path::PathBuf;

        #[test]
        fn test_cd_destination_no_exist() -> Result<(), Box<dyn std::error::Error>> {
            let expected = Err(StatusError { code: 1 });
            let actual = builtins::cd(&["cd".to_string(), "no_exist".to_string()]);

            assert_eq!(expected, actual);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_cd_no_destination() -> Result<(), Box<dyn std::error::Error>> {
            let old_cwd = env::current_dir()?;

            let dirs = UserDirs::new().unwrap();

            let expected = ();
            let expected_cwd = dirs.home_dir();

            let actual = builtins::cd(&["cd".to_string()])?;
            let actual_cwd = env::current_dir().unwrap();

            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            assert_eq!(expected_cwd, actual_cwd);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_cd_directory() -> Result<(), Box<dyn std::error::Error>> {
            let old_cwd = env::current_dir()?;

            let expected = ();
            let expected_cwd = PathBuf::from("./tests").canonicalize()?;

            let actual = builtins::cd(&["cd".to_string(), "tests".to_string()])?;
            let actual_cwd = env::current_dir()?;

            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            assert_eq!(expected_cwd, actual_cwd);

            Ok(())
        }
    }

    // todo: test output to stdout
    mod test_mode {
        use crate::builtins;
        use crate::error::StatusError;
        use crate::history::History;
        use crate::session::{Session, SessionMode};
        use std::io::BufWriter;

        #[test]
        fn test_get_mode_no_set() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut session = Session::new(History::empty(), false, "", SessionMode::Wrapped);

            let expected = Ok(());
            let actual = builtins::mode(&mut out, &mut err, &mut session, &["mode".to_string()]);

            assert_eq!(expected, actual);

            let expected_out = String::from("wrapped\n");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_set_mode() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut session = Session::new(History::empty(), false, "", SessionMode::Wrapped);

            let expected = Ok(());
            let actual = builtins::mode(
                &mut out,
                &mut err,
                &mut session,
                &["mode".to_string(), "normal".to_string()],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from("");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_set_mode_to_current() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut session = Session::new(History::empty(), false, "", SessionMode::Wrapped);

            let expected = Ok(());
            let actual = builtins::mode(
                &mut out,
                &mut err,
                &mut session,
                &["mode".to_string(), "wrapped".to_string()],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from("");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_set_mode_invalid() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut session = Session::new(History::empty(), false, "", SessionMode::Wrapped);

            let expected = Err(StatusError { code: 1 });
            let actual = builtins::mode(
                &mut out,
                &mut err,
                &mut session,
                &["mode".to_string(), "invalid".to_string()],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from("");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_set_mode_frozen() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut session = Session::new(History::empty(), true, "", SessionMode::Wrapped);

            let expected = Err(StatusError { code: 1 });
            let actual = builtins::mode(
                &mut out,
                &mut err,
                &mut session,
                &["mode".to_string(), "normal".to_string()],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from("");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("Error: could not set session mode, session is frozen");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_set_mode_to_current_frozen() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut session = Session::new(History::empty(), true, "", SessionMode::Wrapped);

            let expected = Err(StatusError { code: 1 });
            let actual = builtins::mode(
                &mut out,
                &mut err,
                &mut session,
                &["mode".to_string(), "wrapped".to_string()],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from("");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("Error: could not set session mode, session is frozen");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }
    }

    mod test_history {
        use crate::builtins;
        use crate::error::StatusError;
        use crate::history::{History, HistoryEntry};
        use crate::session::{Session, SessionMode};
        use std::io::BufWriter;

        fn get_history() -> History {
            let mut history = History::empty();

            history.push(HistoryEntry::new(
                "add -A".to_string(),
                Some("git".to_string()),
                SessionMode::Wrapped,
                false,
            ));

            history.push(HistoryEntry::new(
                "mode normal".to_string(),
                None,
                SessionMode::Wrapped,
                true,
            ));

            history.push(HistoryEntry::new(
                "git commit -m 'some commit message'".to_string(),
                None,
                SessionMode::Normal,
                false,
            ));

            history.push(HistoryEntry::new(
                "mode wrapped".to_string(),
                None,
                SessionMode::Normal,
                true,
            ));

            history.push(HistoryEntry::new(
                "clippy".to_string(),
                Some("cargo".to_string()),
                SessionMode::Wrapped,
                false,
            ));

            history
        }

        #[test]
        fn test_history() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut history = get_history();

            let mut session = Session::new(history, false, "git", SessionMode::Wrapped);

            let expected = Ok(());
            let actual =
                builtins::history(&mut out, &mut err, &mut session, &["history".to_string()]);

            assert_eq!(expected, actual);

            let expected_out = String::from("0: git add -A\n1: mode normal\n2: mode wrapped\n");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_unexpected_arg() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let history = get_history();
            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Err(StatusError { code: 1 });
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &["history".to_string(), "unexpected".to_string()],
            );

            assert_eq!(expected, actual);

            assert_eq!(expected, actual);

            let expected_out = String::from("");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut history = get_history();

            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Ok(());
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &["history".to_string(), "filter".to_string()],
            );

            assert_eq!(expected, actual);

            assert_eq!(expected, actual);

            let expected_out = String::from("0: git add -A\n1: mode normal\n2: git commit -m 'some commit message'\n3: mode wrapped\n4: cargo clippy\n");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_mode() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut history = get_history();

            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Ok(());
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &[
                    "history".to_string(),
                    "filter".to_string(),
                    "--mode".to_string(),
                    "wrapped".to_string(),
                ],
            );

            assert_eq!(expected, actual);

            assert_eq!(expected, actual);

            let expected_out = String::from("0: git add -A\n1: cargo clippy\n");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_invalid_mode() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut history = get_history();

            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Err(StatusError { code: 1 });
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &[
                    "history".to_string(),
                    "filter".to_string(),
                    "--mode".to_string(),
                    "invalid".to_string(),
                ],
            );

            assert_eq!(expected, actual);

            assert_eq!(expected, actual);

            let expected_out = String::from("");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_base() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut history = get_history();

            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Ok(());
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &[
                    "history".to_string(),
                    "filter".to_string(),
                    "--base".to_string(),
                    "git".to_string(),
                ],
            );

            assert_eq!(expected, actual);

            assert_eq!(expected, actual);

            let expected_out = String::from("0: mode normal\n1: git commit -m 'some commit message'\n");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }
    }
}
