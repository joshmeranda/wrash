use std::path::PathBuf;

use clap::{Arg, ErrorKind};
use directories::UserDirs;

type BuiltinResult = Result<i32, (i32, String)>;

/// handle_matches is designed to allow for clean and uniform argument handling.
macro_rules! handle_matches {
    ($app:ident, $argv:ident) => {
        match $app.get_matches_from_safe($argv) {
            Err(err) => match err.kind {
                ErrorKind::HelpDisplayed => {
                    println!("{}", err);
                    return Ok(0)
                }
                _ => return Err((1, format!("Error: '{}", err)))
            },
            Ok(m) => m,
        }
    }
}

/// Exit is a builtin for exiting out of the current shell session.
pub fn exit(argv: &[String]) -> BuiltinResult {
    let app = app_from_crate!().name("exit").arg(
        Arg::with_name("code")
            .help("the number to use for the exit status if supplied")
            .default_value("0")
            .validator(|code| {
                match code.parse::<i32>() {
                    Err(err) => Err(format!("could not parse integer from value '{}': {}", code, err)),
                    Ok(n) => if n < 0 {
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
            None => return Err((2, String::from("could not determine the home directory for the current user")))
        };

        dirs.home_dir().to_path_buf()
    };

    if let Err(err) = std::env::set_current_dir(target) {
        eprintln!("Error changing directories: {}", err)
    }

    Ok(0)
}
