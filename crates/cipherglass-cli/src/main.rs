//! `cipherglass` command-line interface.
//!
//! Exit codes: 0 success, 1 usage failure, 2 IO failure.

use std::process::ExitCode;

use clap::Parser;
use clap::error::ErrorKind;

/// Find useful security info in deployed client files.
#[derive(Parser)]
#[command(
    name = "cipherglass",
    version,
    arg_required_else_help = true,
    help_template = "{name} - {about}\n\n{usage-heading} {usage}\n\n{all-args}"
)]
struct Cli {}

const EXIT_USAGE: u8 = 1;
const EXIT_IO: u8 = 2;

fn main() -> ExitCode {
    match Cli::try_parse() {
        Ok(Cli {}) => ExitCode::SUCCESS,
        Err(err) => report(&err),
    }
}

/// Prints a clap outcome and maps it to Cipherglass exit codes. clap's own
/// exit codes (2 for usage errors) are deliberately not used.
fn report(err: &clap::Error) -> ExitCode {
    match err.kind() {
        // Requested output goes to stdout. If stdout cannot be written to
        // (e.g. a closed pipe), that is an IO failure.
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => match err.print() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(EXIT_IO),
        },
        // Usage errors go to stderr. A failure to write to stderr cannot be
        // reported anywhere, so the usage exit code stands.
        _ => {
            let _ = err.print();
            ExitCode::from(EXIT_USAGE)
        }
    }
}
