use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

use crate::{Session, SessionMode, WrashError};
use clap::{Arg, ErrorKind, SubCommand};
use directories::UserDirs;
use regex::Regex;

type BuiltinResult = Result<(), WrashError>;

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

                    return Err(WrashError::NonZeroExit(1));
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

/// With no clear way to distinguish between the exit builtin exiting with a
/// non-zero status code due to bad arguments or proper execution, and no clean
/// and simple way to test a call to `std::process::exit` we wrap this method
/// using `exit` by passing in an `exiter` closure.
fn inner_exit<F: FnOnce(i32)>(argv: &[String], exiter: F) -> BuiltinResult {
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

    exiter(code);

    // this segment is likely only called in tests
    if code == 0 {
        Ok(())
    } else {
        Err(WrashError::NonZeroExit(code))
    }
}

/// Exit is a builtin for exiting out of the current shell session.
pub fn exit(argv: &[String]) -> BuiltinResult {
    // we cannot pas `std::process::exit` directly since we cannot make
    // `inner_exit` take a `FnOnce(i32) -> !` since `!` is an experimental type
    #![allow(clippy::redundant_closure)]
    inner_exit(argv, |n| std::process::exit(n))?;

    unreachable!()
}

/// CD is builtin for changing the current working directory in the shell.
pub fn cd(err_writer: &mut impl Write, argv: &[String]) -> BuiltinResult {
    let app = app_from_crate!()
        .name("cd")
        .about("change the current working directory")
        .arg(Arg::with_name("directory").help("the directory to change into"));

    let matches = handle_matches!(app, argv);

    let target = if matches.is_present("directory") {
        PathBuf::from(matches.value_of("directory").unwrap())
    } else {
        let dirs = match UserDirs::new() {
            Some(dirs) => dirs,
            None => {
                eprintln!("could not determine the home directory for the current user");

                return Err(WrashError::NonZeroExit(2));
            }
        };

        dirs.home_dir().to_path_buf()
    };

    if let Err(err) = std::env::set_current_dir(target) {
        writeln!(err_writer, "Error changing directory: {}", err)?;
        Err(WrashError::FailedIo(err))
    } else {
        Ok(())
    }
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
            writeln!(
                err_writer,
                "Error: could not set session mode, session is frozen"
            )?;
            Err(WrashError::NonZeroExit(1))
        } else {
            Ok(())
        }
    } else {
        writeln!(out_writer, "{}", session.mode())?;

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
            .arg(Arg::with_name("mode-filter").short("m").long("mode").takes_value(true).help("only show commands from the given shell execution mod (only useful"))
            .arg(Arg::with_name("base-filter").short("b").long("base").takes_value(true).help("only show commands whose 'base' matches the given base or have no base (may only be used when '--mode' is 'wrapped')"))
            .arg(Arg::with_name("show-builtin").short("s").long("show-builtin").help("do not ignore builtins and run the same filter checks on builtins as with other commands"))
            .arg(Arg::with_name("pattern").help("the optional regex pattern to search"))
        );

    let matches = handle_matches!(app, argv);

    match matches.subcommand() {
        ("sync", Some(_)) => {
            if let Err(err) = session.history_sync() {
                write!(err_writer, "Error saving to history file: {}", err)?;
            }
        }
        ("filter", Some(sub_matches)) => {
            let base_filter = sub_matches.value_of("base-filter");
            let mode_filter = match sub_matches.value_of("mode-filter") {
                Some(mode) => {
                    if let Ok(parsed) = SessionMode::from_str(mode) {
                        Some(parsed)
                    } else {
                        write!(
                            err_writer,
                            "could not parse value '{}', as SessionMode",
                            mode
                        )?;
                        return Err(WrashError::NonZeroExit(1));
                    }
                }
                None => None,
            };
            let show_builtin = sub_matches.is_present("show-builtin");

            let pattern = if let Some(s) = sub_matches.value_of("pattern") {
                match Regex::new(s) {
                    Ok(r) => Some(r),
                    Err(_) => {
                        write!(err_writer, "invalid regex pattern '{}'", s)?;
                        return Err(WrashError::NonZeroExit(1));
                    }
                }
            } else {
                None
            };

            if base_filter.is_some()
                && mode_filter.is_some()
                && mode_filter.unwrap() != SessionMode::Wrapped
            {
                write!(
                    err_writer,
                    "option '--base' may not be used when '--mode' is not 'wrapped'"
                )?;
                return Err(WrashError::NonZeroExit(1));
            }

            for (i, entry) in session
                .history_iter()
                .filter(|entry| {
                    if entry.is_builtin && !show_builtin {
                        return false;
                    }

                    if let Some(base) = base_filter {
                        if entry.base.is_none() || entry.base.as_ref().unwrap() != base {
                            return false;
                        }
                    }

                    if let Some(mode) = mode_filter {
                        if entry.mode != mode {
                            return false;
                        }
                    }

                    if let Some(pattern) = &pattern {
                        pattern.is_match(entry.get_command().as_str())
                    } else {
                        true
                    }
                })
                .enumerate()
            {
                writeln!(out_writer, "{}: {}", i, entry.get_command())?;
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
                writeln!(out_writer, "{}: {}", i, entry.get_command())?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    mod test_exit {
        use crate::builtins;
        use crate::error::WrashError;

        #[test]
        fn test_exit_no_arg() {
            let expected_exit_code = 0;

            let expected = Ok(());
            let actual = builtins::inner_exit(&["exit".to_string()], |actual_exit_code| {
                assert_eq!(expected_exit_code, actual_exit_code)
            });

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_exit_zero() {
            let expected_exit_code = 0;

            let expected = Ok(());
            let actual = builtins::inner_exit(&["0".to_string()], |actual_exit_code| {
                assert_eq!(expected_exit_code, actual_exit_code)
            });

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_exit_1() {
            let expected_exit_code = 1;

            let expected = Err(WrashError::NonZeroExit(1));
            let actual =
                builtins::inner_exit(&["exit".to_string(), "1".to_string()], |actual_exit_code| {
                    assert_eq!(expected_exit_code, actual_exit_code)
                });

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_exit_neg_1() {
            let expected = Err(WrashError::NonZeroExit(1));
            let actual = builtins::inner_exit(&["exit".to_string(), "nan".to_string()], |_| {});

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_exit_non_number() {
            let expected = Err(WrashError::NonZeroExit(1));
            let actual = builtins::inner_exit(&["exit".to_string(), "nan".to_string()], |_| {});

            assert_eq!(expected, actual);
        }
    }

    mod test_cd {
        use crate::builtins;
        use crate::error::WrashError;
        use directories::UserDirs;
        use std::env;
        use std::io::{BufWriter, Error, ErrorKind};
        use std::path::PathBuf;

        #[test]
        fn test_cd_destination_no_exist() -> Result<(), Box<dyn std::error::Error>> {
            let mut err = BufWriter::new(vec![]);

            let expected = Err(WrashError::FailedIo(Error::new(
                ErrorKind::NotFound,
                "No such file or directory",
            )));
            let actual = builtins::cd(&mut err, &["cd".to_string(), "no_exist".to_string()]);

            assert_eq!(expected, actual);

            let expected_err =
                String::from("Error changing directory: No such file or directory (os error 2)\n");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_cd_no_destination() -> Result<(), Box<dyn std::error::Error>> {
            let mut err = BufWriter::new(vec![]);
            let old_cwd = env::current_dir()?;

            let dirs = UserDirs::new().unwrap();

            let expected = ();
            let expected_cwd = dirs.home_dir();

            let actual = builtins::cd(&mut err, &["cd".to_string()])?;
            let actual_cwd = env::current_dir().unwrap();

            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            assert_eq!(expected_cwd, actual_cwd);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_cd_directory() -> Result<(), Box<dyn std::error::Error>> {
            let mut err = BufWriter::new(vec![]);
            let old_cwd = env::current_dir()?;

            let expected = ();
            let expected_cwd = PathBuf::from("./tests").canonicalize()?;

            let actual = builtins::cd(&mut err, &["cd".to_string(), "tests".to_string()])?;
            let actual_cwd = env::current_dir()?;

            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            assert_eq!(expected_cwd, actual_cwd);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }
    }

    mod test_mode {
        use crate::builtins;
        use crate::error::WrashError;
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

            let expected = Err(WrashError::NonZeroExit(1));
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

            let expected = Err(WrashError::NonZeroExit(1));
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

            let expected_err =
                String::from("Error: could not set session mode, session is frozen\n");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_set_mode_to_current_frozen() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let mut session = Session::new(History::empty(), true, "", SessionMode::Wrapped);

            let expected = Err(WrashError::NonZeroExit(1));
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

            let expected_err =
                String::from("Error: could not set session mode, session is frozen\n");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }
    }

    mod test_history {
        use crate::builtins;
        use crate::error::WrashError;
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

            let history = get_history();

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

            let expected = Err(WrashError::NonZeroExit(1));
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &["history".to_string(), "unexpected".to_string()],
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
        fn test_history_filter() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let history = get_history();

            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Ok(());
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &["history".to_string(), "filter".to_string()],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from(
                "0: git add -A\n1: git commit -m 'some commit message'\n2: cargo clippy\n",
            );
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_show_builtins() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let history = get_history();

            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Ok(());
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &[
                    "history".to_string(),
                    "filter".to_string(),
                    "--show-builtin".to_string(),
                ],
            );

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

            let history = get_history();

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

            let expected_out = String::from("0: git add -A\n1: cargo clippy\n");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_mode_show_builtins() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let history = get_history();

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
                    "--show-builtin".to_string(),
                ],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from("0: git add -A\n1: mode normal\n2: cargo clippy\n");
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

            let history = get_history();

            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Err(WrashError::NonZeroExit(1));
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

            let expected_out = String::from("");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("could not parse value 'invalid', as SessionMode");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_base() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let history = get_history();

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

            let expected_out = String::from("0: git add -A\n");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_base_show_builtins() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let history = get_history();

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
                    "--show-builtin".to_string(),
                ],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from("0: git add -A\n");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_base_with_filter_normal() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let history = get_history();

            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Err(WrashError::NonZeroExit(1));
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &[
                    "history".to_string(),
                    "filter".to_string(),
                    "--base".to_string(),
                    "git".to_string(),
                    "--mode".to_string(),
                    "normal".to_string(),
                ],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from("");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err =
                String::from("option '--base' may not be used when '--mode' is not 'wrapped'");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_base_with_filter_wrapped() -> Result<(), Box<dyn std::error::Error>>
        {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let history = get_history();

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
                    "--mode".to_string(),
                    "wrapped".to_string(),
                ],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from("0: git add -A\n");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_invalid_regex() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let history = get_history();

            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Err(WrashError::NonZeroExit(1));
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &[
                    "history".to_string(),
                    "filter".to_string(),
                    "i have an unclosed brace [".to_string(),
                ],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from("");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("invalid regex pattern 'i have an unclosed brace ['");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_regex_contains_git() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let history = get_history();

            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Ok(());
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &[
                    "history".to_string(),
                    "filter".to_string(),
                    "git".to_string(),
                ],
            );

            assert_eq!(expected, actual);

            let expected_out =
                String::from("0: git add -A\n1: git commit -m 'some commit message'\n");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }

        #[test]
        fn test_history_filter_regex_no_middle() -> Result<(), Box<dyn std::error::Error>> {
            let mut out = BufWriter::new(vec![]);
            let mut err = BufWriter::new(vec![]);

            let history = get_history();

            let mut session = Session::new(history, false, "", SessionMode::Wrapped);

            let expected = Ok(());
            let actual = builtins::history(
                &mut out,
                &mut err,
                &mut session,
                &[
                    "history".to_string(),
                    "filter".to_string(),
                    "car.*ppy".to_string(),
                ],
            );

            assert_eq!(expected, actual);

            let expected_out = String::from("0: cargo clippy\n");
            let actual_out = String::from_utf8(out.into_inner()?).unwrap();

            assert_eq!(expected_out, actual_out);

            let expected_err = String::from("");
            let actual_err = String::from_utf8(err.into_inner()?).unwrap();

            assert_eq!(expected_err, actual_err);

            Ok(())
        }
    }
}
