//! `trunk status` — print the configured trunk branch.

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::git;

pub fn run() -> Result<()> {
    println!("{}", git::target_branch().green());
    Ok(())
}
