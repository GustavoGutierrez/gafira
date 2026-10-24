#![forbid(unsafe_code)]

//! `gafira-cli`: minimal command-line entrypoint (std only, no UI dependencies).

use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

const NAME: &str = "gafira-cli";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const USAGE: &str = "Usage: gafira-cli [--help | --version]\n\n\
Options:\n  --help       Print this help and exit\n  --version    Print the version and exit\n";

/// What the process should do for a given argument list.
#[derive(Debug, PartialEq, Eq)]
enum Action {
    Version,
    Help,
    /// Usage error; carries the offending argument, or `None` when none was given.
    Usage(Option<OsString>),
}

/// Pure argument parser: `--version` and `--help` are only valid as the sole argument.
fn parse(args: &[OsString]) -> Action {
    match args {
        [] => Action::Usage(None),
        [only] if only == "--version" => Action::Version,
        [only] if only == "--help" => Action::Help,
        [first, ..] if first == "--version" || first == "--help" => {
            Action::Usage(args.get(1).cloned())
        }
        [first, ..] => Action::Usage(Some(first.clone())),
    }
}

fn emit(mut out: impl Write, text: &str) -> io::Result<()> {
    out.write_all(text.as_bytes())?;
    out.flush()
}

fn run(args: &[OsString]) -> io::Result<ExitCode> {
    match parse(args) {
        Action::Version => {
            emit(io::stdout().lock(), &format!("{NAME} {VERSION}\n")).map(|()| ExitCode::SUCCESS)
        }
        Action::Help => emit(io::stdout().lock(), USAGE).map(|()| ExitCode::SUCCESS),
        Action::Usage(culprit) => {
            let message = match culprit {
                Some(arg) => format!(
                    "error: unexpected argument '{}'\n{USAGE}",
                    arg.to_string_lossy()
                ),
                None => USAGE.to_owned(),
            };
            emit(io::stderr().lock(), &message).map(|()| ExitCode::from(2))
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    run(&args).unwrap_or(ExitCode::FAILURE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn args(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_version() {
        assert_eq!(parse(&args(&["--version"])), Action::Version);
    }

    #[test]
    fn parses_help() {
        assert_eq!(parse(&args(&["--help"])), Action::Help);
    }

    #[test]
    fn no_arguments_is_usage_error_without_culprit() {
        assert_eq!(parse(&args(&[])), Action::Usage(None));
    }

    #[test]
    fn unknown_argument_is_reported() {
        assert_eq!(
            parse(&args(&["--bogus"])),
            Action::Usage(Some(OsString::from("--bogus")))
        );
    }

    #[test]
    fn extra_argument_after_valid_flag_is_reported() {
        assert_eq!(
            parse(&args(&["--version", "extra"])),
            Action::Usage(Some(OsString::from("extra")))
        );
    }
}
