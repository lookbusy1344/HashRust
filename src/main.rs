use anyhow::Result;
use pico_args::Arguments;

use cli::{process_command_line, show_help};
use core::worker_func;

mod cli;
mod core;
mod hash;
mod io;
mod progress;
#[cfg(test)]
mod unit_tests;

fn main() -> Result<()> {
    let result = worker_main();

    if let Err(e) = result {
        show_help(true);
        println!();
        return Err(e);
    }

    Ok(())
}

fn worker_main() -> Result<()> {
    let mut pargs = Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        show_help(true);
        return Ok(());
    }

    let config = process_command_line(pargs)?;
    worker_func(&config)
}
