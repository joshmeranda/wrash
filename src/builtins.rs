use std::path::PathBuf;

use clap::{Arg, ErrorKind};

pub fn exit(argv: &[String]) {
    let app = app_from_crate!().name("exit").arg(
        Arg::with_name("code")
            .help("the number to use for the exit status if supplied")
            .default_value("0")
            .validator(|code| {
                if code.parse::<i32>().is_err() {
                    Err(String::from("exit code must be a non-negative int"))
                } else {
                    Ok(())
                }
            }),
    );

    let matches = match app.get_matches_from_safe(argv) {
        Err(err) => match err.kind {
            ErrorKind::HelpDisplayed => {
                println!("{}", err);
                return;
            }
            _ => {
                eprintln!("Error: '{}", err);
                return;
            }
        },
        Ok(m) => m,
    };

    let code: i32 = matches.value_of("code").unwrap().parse().unwrap();

    std::process::exit(code);
}

pub fn cd(argv: &[String]) {
    let app = app_from_crate!().name("cd").arg(
        Arg::with_name("directory")
            .help("the directory to change into")
            .default_value(
                /* todo: get the users home directory instead  (paths create) */ "$HOME",
            )
            .validator(|dir| {
                if !PathBuf::from(dir.as_str()).is_dir() {
                    Err(format!("no such file or directory '{}'", dir))
                } else {
                    Ok(())
                }
            }),
    );

    let matches = match app.get_matches_from_safe(argv) {
        Err(err) => match err.kind {
            ErrorKind::HelpDisplayed => {
                println!("{}", err);
                return;
            }
            _ => {
                eprintln!("Error: '{}", err);
                return;
            }
        },
        Ok(m) => m,
    };

    let target = matches.value_of("directory").unwrap();

    if let Err(err) = std::env::set_current_dir(target) {
        eprintln!("Error changing directories: {}", err)
    }
}
