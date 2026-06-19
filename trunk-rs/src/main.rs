//! `trunk` — a small helper for working with a configurable trunk branch.
//!
//! A Rust port of the original fish script. Same commands, same git config
//! key (`zemzale.release`); the picker (`skim`) and spinners (`indicatif`) are
//! now in-process instead of shelling out to `fzf` and `gum`.

mod commands;
mod git;
mod restore;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "trunk", about = "Work with your trunk branch")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Pick a new trunk branch and store it in git config
    Change,
    /// Print the configured trunk branch
    Status,
    /// Sync trunk, then return to the current branch
    Pull,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Some(Command::Change) => commands::change::run(),
        Some(Command::Status) => commands::status::run(),
        Some(Command::Pull) => commands::pull::run(),
        // No subcommand: sync trunk and clean up merged branches.
        None => commands::run::run(),
    }
}
