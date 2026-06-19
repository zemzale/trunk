//! Default command — sync trunk and clean up merged/stale branches.
//!
//! Like the original script, this ends on the trunk branch.

use anyhow::Result;

use crate::restore::Restore;
use crate::{git, ui};

pub fn run() -> Result<()> {
    let target = git::target_branch();
    let current = git::current_branch()?;
    let dirty = git::is_dirty()?;

    if dirty {
        ui::step("Stashing changes", git::stash)?;
    }

    // Success ends on trunk (matching the original). The guard only fires on
    // an error, returning us to `current` and restoring the stash so changes
    // are never stranded.
    let mut guard = Restore::new(&current, dirty);

    ui::step(&format!("Switching to {target} branch"), || {
        git::switch(&target)
    })?;
    ui::step("Pulling latest changes", git::pull)?;
    cleanup_merged(&target)?;
    ui::step("Cleaning up remote branches", git::remote_prune)?;
    if dirty {
        ui::step("Popping changes", git::stash_pop)?;
    }

    guard.disarm();
    Ok(())
}

/// Delete every branch already merged into `target`. Deletion is best-effort:
/// a branch that refuses to delete is reported but does not abort the run.
fn cleanup_merged(target: &str) -> Result<()> {
    let branches = git::merged_branches(target)?;

    ui::step("Cleaning up merged branches", || {
        let mut report = Vec::new();
        for branch in &branches {
            match git::delete_branch(branch) {
                Ok(out) => report.push(out),
                Err(e) => report.push(format!("skipped {branch}: {e}")),
            }
        }
        Ok(report.join("\n"))
    })?;

    Ok(())
}
