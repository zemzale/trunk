//! `trunk change` — pick a new trunk branch via a fuzzy finder.
//!
//! Replaces the `fzf` pipeline with the in-process `skim` picker, keeping the
//! `git show --shortstat` preview.

use std::process::Command;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use skim::prelude::*;

use crate::git;

pub fn run() -> Result<()> {
    let branches = git::branch_list()?;
    if branches.is_empty() {
        println!("No branches found");
        return Ok(());
    }

    let options = SkimOptionsBuilder::default()
        .reverse(true)
        .preview_fn(PreviewCallback::from(
            |items: Vec<Arc<dyn SkimItem>>| -> Vec<String> {
                items
                    .iter()
                    .flat_map(|item| preview_lines(item.text().as_ref()))
                    .collect()
            },
        ))
        .build()
        .map_err(|e| anyhow!("failed to build picker options: {e}"))?;

    let output =
        Skim::run_items(options, branches).map_err(|e| anyhow!("branch picker failed: {e}"))?;

    if output.is_abort || output.selected_items.is_empty() {
        println!("No branch selected");
        return Ok(());
    }

    let selected = output.selected_items[0].output().to_string();
    git::set_release(&selected)?;
    println!("Trunk set to {selected}");
    Ok(())
}

/// Preview body for a branch: `git show --shortstat <branch>`.
fn preview_lines(branch: &str) -> Vec<String> {
    match Command::new("git")
        .args(["show", "--shortstat", branch])
        .output()
    {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
        Err(e) => vec![format!("failed to preview {branch}: {e}")],
    }
}
