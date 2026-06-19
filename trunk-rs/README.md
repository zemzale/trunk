# trunk-rs

A Rust port of the `trunk` fish script. Same commands and the same git config
key (`zemzale.release`), with the `fzf`/`gum` dependencies replaced by
in-process Rust (`skim` for the picker, `indicatif` for spinners).

## Commands

| Command        | Behaviour                                                            |
| -------------- | ------------------------------------------------------------------- |
| `trunk`        | Stash (if dirty) → switch to trunk → pull → delete merged branches → prune remote → pop. Ends on trunk. |
| `trunk pull`   | Sync trunk, then return to the branch you started on (or just pull if already on trunk). |
| `trunk change` | Fuzzy-pick a new trunk branch (with `git show --shortstat` preview) and save it to `zemzale.release`. |
| `trunk status` | Print the configured trunk branch.                                  |

The trunk branch comes from `git config zemzale.release`, defaulting to `main`.

## Improvements over the script

- **Conditional stash:** only stashes when the tree is dirty, and only pops a
  stash it created — the original could pop an unrelated older stash.
- **Crash-safe:** if a step fails midway, it returns you to your original
  branch and restores the stash before exiting (see `src/restore.rs`).
- **Stops on failure:** a failed `switch` no longer lets `pull` run on the
  wrong branch.
- **Correct merged-branch filter:** honours the configured trunk instead of a
  hardcoded, malformed `main` regex.

## Build & install

```sh
make install      # builds --release, installs to ~/.local/bin/trunk-claude
make uninstall    # removes it
```

Override the location or name with `make install BINDIR=/usr/local/bin NAME=trunk`.

Requires the `git` binary on `PATH`. No `fzf` or `gum` needed.
