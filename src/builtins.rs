use std::path::Path;

pub fn exit(_argv: &[String]) {
    std::process::exit(0);
}

pub fn cd(argv: &[String]) {
    let target = Path::new(argv[1].as_str());

    if let Err(err) = std::env::set_current_dir(target) {
        eprintln!("Error changing directories: {}", err)
    }
}