#![forbid(unsafe_code)]
use anyhow::Result;
use pico_args::Arguments;

use cli::{process_command_line, show_help};
use core::{FileHashError, worker_func};

mod cli;
mod core;
mod hash;
mod io;
mod progress;
#[cfg(test)]
mod unit_tests;

fn main() -> Result<()> {
    let result = worker_main();

    if let Err(ref e) = result {
        // File-hash failures already print per-file errors; suppress the help banner.
        // Config/arg errors do need the banner so the user knows how to fix them.
        if e.downcast_ref::<FileHashError>().is_none() {
            show_help(true, &mut std::io::stderr());
            eprintln!();
        }
        return result;
    }

    Ok(())
}

fn worker_main() -> Result<()> {
    let mut pargs = Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        show_help(true, &mut std::io::stdout());
        return Ok(());
    }

    let config = process_command_line(pargs)?;
    worker_func(&config)
}
