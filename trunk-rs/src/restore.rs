//! Safety net that returns the repo to a sane state if a command aborts midway.

use crate::git;

/// On drop (i.e. an error caused an early return) this switches back to the
/// original branch and pops the stash if one was created. Call [`disarm`] on
/// the success path so it becomes a no-op.
///
/// [`disarm`]: Restore::disarm
pub struct Restore {
    original: String,
    stashed: bool,
    armed: bool,
}

impl Restore {
    pub fn new(original: &str, stashed: bool) -> Self {
        Self {
            original: original.to_string(),
            stashed,
            armed: true,
        }
    }

    pub fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for Restore {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }

        eprintln!("Restoring original state…");
        if let Err(e) = git::switch(&self.original) {
            eprintln!("  failed to switch back to {}: {e}", self.original);
        }
        if self.stashed && let Err(e) = git::stash_pop() {
            eprintln!("  failed to pop stash: {e}");
        }
    }
}
