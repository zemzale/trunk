//! `trunk pull` — sync the trunk branch, then return to where you started.

use anyhow::Result;

use crate::restore::Restore;
use crate::{git, ui};

pub fn run() -> Result<()> {
    let target = git::target_branch();
    let current = git::current_branch()?;

    // Already on trunk: nothing to stash or switch, just pull.
    if current == target {
        ui::step("Pulling latest changes", git::pull)?;
        return Ok(());
    }

    // Only stash when there is something to stash, so we never pop an
    // unrelated older stash later (a bug in the original script).
    let dirty = git::is_dirty()?;
    if dirty {
        ui::step("Stashing changes", git::stash)?;
    }

    // If any step below fails, this returns us to `current` and restores the
    // stash before the error propagates.
    let mut guard = Restore::new(&current, dirty);

    ui::step(&format!("Switching to {target} branch"), || {
        git::switch(&target)
    })?;
    ui::step("Pulling latest changes", git::pull)?;
    ui::step(&format!("Switching back to {current}"), || {
        git::switch(&current)
    })?;
    if dirty {
        ui::step("Popping changes", git::stash_pop)?;
    }

    guard.disarm();
    Ok(())
}
