//! Thin wrappers around the `git` binary.
//!
//! Everything shells out to `git` so the user's full configuration applies:
//! credential helpers, ssh config, `pull.rebase`, hooks, aliases, signing, etc.
//! A library like libgit2/gix would bypass all of that and force us to
//! reimplement porcelain such as `pull` and `stash`.

use std::process::Command;

use anyhow::{Context, Result, bail};

/// Run `git` with `args`, returning trimmed stdout on success.
///
/// On a non-zero exit the error carries git's stderr so the failing step is
/// obvious.
pub fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .with_context(|| format!("failed to run `git {}`", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`git {}` failed: {}", args.join(" "), stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim_end().to_string())
}

/// The configured trunk branch (`zemzale.release`), defaulting to `main`.
pub fn target_branch() -> String {
    // `git config --get` exits non-zero when the key is unset, so `.ok()`
    // collapses that to the default.
    git(&["config", "--get", "zemzale.release"])
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "main".to_string())
}

pub fn current_branch() -> Result<String> {
    git(&["branch", "--show-current"])
}

/// True if the working tree has uncommitted changes.
pub fn is_dirty() -> Result<bool> {
    Ok(!git(&["status", "--porcelain"])?.is_empty())
}

pub fn pull() -> Result<String> {
    git(&["pull"])
}

pub fn switch(branch: &str) -> Result<String> {
    git(&["switch", branch])
}

pub fn stash() -> Result<String> {
    git(&["stash"])
}

pub fn stash_pop() -> Result<String> {
    git(&["stash", "pop"])
}

pub fn remote_prune() -> Result<String> {
    git(&["remote", "prune", "origin"])
}

/// Clean local branch names (no `*` marker), one per branch.
pub fn branch_list() -> Result<Vec<String>> {
    let out = git(&["branch", "--format=%(refname:short)"])?;
    Ok(out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// Branches merged into the current HEAD, excluding the checked-out branch
/// (the `* ` line) and `keep` (the trunk we just synced).
pub fn merged_branches(keep: &str) -> Result<Vec<String>> {
    let out = git(&["branch", "--merged"])?;
    Ok(out
        .lines()
        .filter(|l| !l.starts_with('*'))
        .map(|l| l.trim().to_string())
        .filter(|b| !b.is_empty() && b != keep)
        .collect())
}

pub fn delete_branch(branch: &str) -> Result<String> {
    git(&["branch", "-d", branch])
}

/// Set `zemzale.release` to `branch`, replacing any existing value(s).
pub fn set_release(branch: &str) -> Result<()> {
    // Best-effort unset (errors when unset) then add, mirroring the original
    // script's `--unset-all` + `--add`.
    let _ = git(&["config", "--unset-all", "zemzale.release"]);
    git(&["config", "--add", "zemzale.release", branch])?;
    Ok(())
}
