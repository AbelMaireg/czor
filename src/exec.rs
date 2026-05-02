use anyhow::{bail, Context, Result};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::layout::{weights_to_split_percentages, Direction, Layout};

static DEBUG: AtomicBool = AtomicBool::new(false);

pub fn set_debug(enabled: bool) {
    DEBUG.store(enabled, Ordering::Relaxed);
}

fn run_tmux(args: &[&str]) -> Result<String> {
    if DEBUG.load(Ordering::Relaxed) {
        eprintln!("+ tmux {}", args.join(" "));
    }
    let output = Command::new("tmux")
        .args(args)
        .output()
        .with_context(|| format!("Failed to run tmux {}", args[0]))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("tmux {} failed: {}", args[0], stderr);
    }

    String::from_utf8(output.stdout)
        .with_context(|| format!("Invalid UTF-8 in tmux {} output", args[0]))
        .map(|s| s.trim().to_string())
}

/// Execute a layout in the current tmux window
pub fn execute(layout: &Layout) -> Result<()> {
    let current_pane = run_tmux(&["display-message", "-p", "#{pane_id}"])?;
    execute_node(layout, &current_pane)
}

fn execute_node(node: &Layout, target_pane: &str) -> Result<()> {
    match node {
        Layout::Pane => Ok(()),
        Layout::Split {
            direction,
            children,
        } => {
            if children.is_empty() {
                return Ok(());
            }

            if children.len() == 1 {
                return execute_node(&children[0].1, target_pane);
            }

            let weights: Vec<u32> = children.iter().map(|(w, _)| *w).collect();
            let percentages = weights_to_split_percentages(&weights);

            let dir_flag = match direction {
                Direction::Horizontal => "-h",
                Direction::Vertical => "-v",
            };

            // First child inherits the target pane.
            // Each subsequent split operates on the most recently created pane,
            // because the cascading percentages assume that geometry.
            let mut pane_ids = vec![target_pane.to_string()];
            let mut split_target = target_pane.to_string();

            for pct in &percentages {
                let new_pane_id = split_pane(&split_target, dir_flag, *pct)?;
                pane_ids.push(new_pane_id.clone());
                split_target = new_pane_id;
            }

            for (i, (_, child)) in children.iter().enumerate() {
                execute_node(child, &pane_ids[i])?;
            }

            Ok(())
        }
    }
}

fn split_pane(target_pane: &str, dir_flag: &str, percentage: u8) -> Result<String> {
    let pct = percentage.to_string();
    run_tmux(&[
        "split-window",
        dir_flag,
        "-p",
        &pct,
        "-t",
        target_pane,
        "-P",
        "-F",
        "#{pane_id}",
    ])
}
