use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);

    match args.next().as_deref() {
        Some("--help") => help_trunk(),
        Some("change") => change_trunk(),
        Some("status") => status_trunk(),
        Some("pull") => update_trunk(),
        _ => run_trunk(),
    }
}

fn help_trunk() -> Result<()> {
    println!(
        "trunk-codex\n\nUsage:\n  trunk-codex\n  trunk-codex pull\n  trunk-codex change\n  trunk-codex status\n  trunk-codex --help\n\nCommands:\n  pull     Update the configured trunk branch, then return to the current branch\n  change   Select and store the trunk branch in git config\n  status   Print the configured trunk branch\n  --help   Print this help text\n\nDefault command:\n  Stash changes, switch to the configured trunk branch, pull, delete merged branches, prune origin, and restore stashed changes\n"
    );
    Ok(())
}

fn update_trunk() -> Result<()> {
    let target_branch = target_branch()?;
    let current_branch = current_branch()?;

    if current_branch == target_branch {
        run_git_spinner(["pull"], "Pulling latest changes")?;
        return Ok(());
    }

    let stashed = stash_changes("Stashing changes")?;
    let mut first_error = None;

    if let Err(err) = run_git_spinner(
        ["switch", target_branch.as_str()],
        &format!("Switching to {target_branch} branch"),
    ) {
        first_error = Some(err);
    }

    if first_error.is_none() {
        if let Err(err) = run_git_spinner(["pull"], "Pulling latest changes") {
            first_error = Some(err);
        }
    }

    if let Err(err) = run_git_spinner(
        ["switch", current_branch.as_str()],
        &format!("Switching back to {current_branch}"),
    ) {
        if first_error.is_none() {
            first_error = Some(err);
        }
    }

    if let Err(err) = pop_stash(stashed, "Popping changes") {
        if first_error.is_none() {
            first_error = Some(err);
        }
    }

    match first_error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

fn run_trunk() -> Result<()> {
    let target_branch = target_branch()?;
    let stashed = stash_changes("Stashing changes")?;
    let result = (|| {
        run_git_spinner(
            ["switch", target_branch.as_str()],
            &format!("Switching to {target_branch} branch"),
        )?;
        run_git_spinner(["pull"], "Pulling latest changes")?;
        cleanup_merged_branches(&target_branch)?;
        run_git_spinner(["remote", "prune", "origin"], "Cleaning up remote branches")?;
        Ok(())
    })();

    let pop_result = pop_stash(stashed, "Popping changes");

    match (result, pop_result) {
        (Err(err), _) => Err(err),
        (Ok(()), Err(err)) => Err(err),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn change_trunk() -> Result<()> {
    let branches = git_output(["branch", "--format=%(refname:short)"])?;
    let selected = run_fzf(&branches)?;

    let Some(new_trunk) = selected else {
        println!("No branch selected");
        return Ok(());
    };

    run_git([
        "config",
        "--replace-all",
        "zemzale.release",
        new_trunk.as_str(),
    ])?;
    Ok(())
}

fn status_trunk() -> Result<()> {
    println!("{}", target_branch()?);
    Ok(())
}

fn target_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["config", "--get", "zemzale.release"])
        .output()
        .context("failed to read git config zemzale.release")?;

    if !output.status.success() {
        return Ok("main".to_string());
    }

    let branch = String::from_utf8(output.stdout)
        .context("git config zemzale.release was not valid UTF-8")?
        .trim()
        .to_string();

    if branch.is_empty() {
        Ok("main".to_string())
    } else {
        Ok(branch)
    }
}

fn current_branch() -> Result<String> {
    let branch = git_output(["branch", "--show-current"])?;
    let branch = branch.trim().to_string();

    if branch.is_empty() {
        bail!("cannot determine current branch; detached HEAD is not supported by trunk pull");
    }

    Ok(branch)
}

fn cleanup_merged_branches(target_branch: &str) -> Result<()> {
    let merged: Vec<String> = git_output(["branch", "--merged"])?
        .lines()
        .map(parse_branch_line)
        .filter(|branch| !branch.is_empty() && branch != target_branch)
        .collect();

    let skipped = cleanup_merged_worktrees(&merged)?;

    let outputs = with_spinner("Cleaning up merged branches", || {
        let mut outputs = Vec::new();

        for branch in merged.iter().filter(|branch| !skipped.contains(branch)) {
            outputs.push(run_git(["branch", "-d", branch.as_str()])?);
        }

        Ok(outputs)
    })?;

    for output in outputs {
        print_output(&output)?;
    }

    Ok(())
}

/// Removes worktrees that have a merged branch checked out so the branch can be deleted.
/// Returns the branches whose worktree could not be removed (e.g. uncommitted changes).
fn cleanup_merged_worktrees(merged: &[String]) -> Result<Vec<String>> {
    run_git(["worktree", "prune"])?;

    let worktrees = parse_worktrees(&git_output(["worktree", "list", "--porcelain"])?);
    let mut skipped = Vec::new();

    // The first entry is the main worktree, which git cannot remove.
    for worktree in worktrees.iter().skip(1) {
        let Some(branch) = worktree.branch.as_deref() else {
            continue;
        };

        if !merged.iter().any(|merged| merged == branch) {
            continue;
        }

        let output = with_spinner(&format!("Removing worktree {}", worktree.path), || {
            run_command(
                "git",
                &collect_args(["worktree", "remove", worktree.path.as_str()]),
            )
        })?;

        if !output.status.success() {
            print_output(&output)?;
            eprintln!(
                "Skipping branch {branch}: could not remove worktree {}",
                worktree.path
            );
            skipped.push(branch.to_string());
        }
    }

    Ok(skipped)
}

struct Worktree {
    path: String,
    branch: Option<String>,
}

fn parse_worktrees(porcelain: &str) -> Vec<Worktree> {
    let mut worktrees = Vec::new();

    for line in porcelain.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            worktrees.push(Worktree {
                path: path.to_string(),
                branch: None,
            });
        } else if let Some(branch) = line.strip_prefix("branch ") {
            if let Some(worktree) = worktrees.last_mut() {
                worktree.branch = Some(branch.trim_start_matches("refs/heads/").to_string());
            }
        }
    }

    worktrees
}

fn parse_branch_line(line: &str) -> String {
    line.trim()
        .trim_start_matches('*')
        .trim_start_matches('+')
        .trim()
        .to_string()
}

fn stash_changes(title: &str) -> Result<Option<String>> {
    let before = stash_ref()?;
    run_git_spinner(["stash", "push", "-m", "trunk: auto-stash"], title)?;
    let after = stash_ref()?;

    if after.is_some() && after != before {
        Ok(after)
    } else {
        Ok(None)
    }
}

fn pop_stash(stashed: Option<String>, title: &str) -> Result<()> {
    let Some(stashed_ref) = stashed else {
        return Ok(());
    };

    let current_ref = stash_ref()?;
    if current_ref.as_deref() != Some(stashed_ref.as_str()) {
        bail!("refusing to pop stash because refs/stash changed while trunk was running");
    }

    run_git_spinner(["stash", "pop"], title)?;
    Ok(())
}

fn stash_ref() -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["rev-parse", "-q", "--verify", "refs/stash"])
        .output()
        .context("failed to inspect git stash")?;

    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(
        String::from_utf8(output.stdout)
            .context("git stash ref was not valid UTF-8")?
            .trim()
            .to_string(),
    ))
}

fn run_fzf(input: &str) -> Result<Option<String>> {
    let mut child = Command::new("fzf")
        .args([
            "--layout",
            "reverse",
            "--preview",
            "git show --shortstat {}",
            "--preview-window",
            "down",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to start fzf")?;

    child
        .stdin
        .as_mut()
        .context("failed to open fzf stdin")?
        .write_all(input.as_bytes())
        .context("failed to write branch list to fzf")?;

    let output = child.wait_with_output().context("failed to wait for fzf")?;
    if !output.status.success() {
        return Ok(None);
    }

    let selected = String::from_utf8(output.stdout)
        .context("fzf output was not valid UTF-8")?
        .trim()
        .to_string();

    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(selected))
    }
}

fn run_git_spinner<I, S>(args: I, title: &str) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args = collect_args(args);
    let output = with_spinner(title, || run_command("git", &args))?;
    print_output(&output)?;
    ensure_success("git", &args, &output)?;
    Ok(output)
}

fn run_git<I, S>(args: I) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args = collect_args(args);
    let output = run_command("git", &args)?;
    ensure_success("git", &args, &output)?;
    Ok(output)
}

fn git_output<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args = collect_args(args);
    let output = run_command("git", &args)?;
    ensure_success("git", &args, &output)?;
    String::from_utf8(output.stdout).context("git output was not valid UTF-8")
}

fn run_command(program: &str, args: &[OsString]) -> Result<Output> {
    Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to run {}", command_label(program, args)))
}

fn collect_args<I, S>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    args.into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect()
}

fn ensure_success(program: &str, args: &[OsString], output: &Output) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }

    Err(anyhow!(
        "{} failed with {}",
        command_label(program, args),
        output.status
    ))
}

fn print_output(output: &Output) -> Result<()> {
    io::stdout()
        .write_all(&output.stdout)
        .context("failed to write command stdout")?;
    io::stderr()
        .write_all(&output.stderr)
        .context("failed to write command stderr")?;
    Ok(())
}

fn with_spinner<T>(title: &str, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    spinner.set_message(title.to_string());
    spinner.enable_steady_tick(Duration::from_millis(100));

    let result = f();
    spinner.finish_and_clear();
    result
}

fn command_label(program: &str, args: &[OsString]) -> String {
    let mut parts = vec![program.to_string()];
    parts.extend(args.iter().map(|arg| arg.to_string_lossy().into_owned()));
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_worktree_porcelain() {
        let porcelain = "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /repo-feat\nHEAD def\nbranch refs/heads/feat/x\n\nworktree /repo-detached\nHEAD 123\ndetached\n";
        let worktrees = parse_worktrees(porcelain);

        assert_eq!(worktrees.len(), 3);
        assert_eq!(worktrees[0].path, "/repo");
        assert_eq!(worktrees[0].branch.as_deref(), Some("main"));
        assert_eq!(worktrees[1].path, "/repo-feat");
        assert_eq!(worktrees[1].branch.as_deref(), Some("feat/x"));
        assert_eq!(worktrees[2].branch, None);
    }
}
