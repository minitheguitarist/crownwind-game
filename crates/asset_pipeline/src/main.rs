//! Validate Crownwind v1 content. Run with --help for the command syntax.

mod manifest;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use manifest::Kind;

const USAGE: &str = "Usage: asset_pipeline [animation FILE [ATLAS.png] | level FILE]\n\
    With no arguments, validate *.json in content/characters and content/levels.";

fn check_file(kind: Kind, path: &Path) -> Result<(), String> {
    let data = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    manifest::validate(kind, &data).map_err(|e| format!("{}: {e}", path.display()))
}

// Only the PNG header is needed to cross-check the manifest's atlas layout.
fn atlas_size(path: &Path) -> Result<(u32, u32), String> {
    let mut header = [0_u8; 33];
    fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if !header.starts_with(b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR") {
        return Err(format!("{}: invalid PNG signature or IHDR", path.display()));
    }
    let width = u32::from_be_bytes(header[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(header[20..24].try_into().unwrap());
    if [width, height]
        .iter()
        .any(|&n| n == 0 || n > i32::MAX as u32)
    {
        return Err(format!(
            "{}: invalid PNG dimensions {width}×{height}",
            path.display()
        ));
    }
    Ok((width, height))
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
        [kind, path, atlas] if kind == "animation" => atlas_size(Path::new(atlas))
            .and_then(|size| check_file(Kind::Atlas(size), Path::new(path))),
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
