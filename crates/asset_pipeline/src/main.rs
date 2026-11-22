//! Validate Crownwind v1 content. Run with --help for the command syntax.

mod manifest;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use manifest::Kind;

const USAGE: &str = "Usage: asset_pipeline [animation FILE | level FILE]\n\
    With no arguments, validate *.json in content/characters and content/levels.";

fn check_file(kind: Kind, path: &Path) -> Result<(), String> {
    let data = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    manifest::validate(kind, &data).map_err(|e| format!("{}: {e}", path.display()))
}

fn check_directory(kind: Kind, dir: &Path) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.extension() == Some(OsStr::new("json")) {
            paths.push(path);
        }
    }
    paths.sort();
    for path in paths {
        check_file(kind, &path)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [] => check_directory(Kind::Animation, Path::new("content/characters"))
            .and_then(|()| check_directory(Kind::Level, Path::new("content/levels"))),
        [flag] if flag == "--help" || flag == "-h" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        [kind, path] if kind == "animation" => check_file(Kind::Animation, Path::new(path)),
        [kind, path] if kind == "level" => check_file(Kind::Level, Path::new(path)),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(issue) => {
            eprintln!("{issue}");
            ExitCode::FAILURE
        }
    }
}
