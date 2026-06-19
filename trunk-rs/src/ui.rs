//! Spinner output, replacing `gum spin --show-output`.

use std::time::Duration;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

/// Run `f` while showing a spinner labelled `title`.
///
/// On success the line is replaced with `✓ title` and any non-empty command
/// output is printed below it (the `--show-output` behaviour). On failure the
/// line becomes `✗ title` and the error propagates.
pub fn step(title: &str, f: impl FnOnce() -> Result<String>) -> Result<String> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▱▰▰", "▱▱▰", "▱▱▱"]),
    );
    pb.set_message(title.to_string());
    pb.enable_steady_tick(Duration::from_millis(120));

    let result = f();

    match &result {
        Ok(out) => {
            pb.finish_with_message(format!("✓ {title}"));
            let out = out.trim();
            if !out.is_empty() {
                println!("{out}");
            }
        }
        Err(_) => pb.abandon_with_message(format!("✗ {title}")),
    }

    result
}
