use anyhow::{bail, Context, Result};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::layout::{weights_to_split_percentages, Direction, Layout};

static DEBUG: AtomicBool = AtomicBool::new(false);

pub fn set_debug(enabled: bool) {
    DEBUG.store(enabled, Ordering::Relaxed);
}

fn debug_log(args: &[&str]) {
    if DEBUG.load(Ordering::Relaxed) {
        eprintln!("+ tmux {}", args.join(" "));
    }
}

/// Execute a layout in the current tmux window
pub fn execute(layout: &Layout) -> Result<()> {
    // Get the current pane ID
    let current_pane = get_current_pane_id()?;
    execute_node(layout, &current_pane)?;
    Ok(())
}

/// Get the current pane ID
fn get_current_pane_id() -> Result<String> {
    let args = ["display-message", "-p", "#{pane_id}"];
    debug_log(&args);
    let output = Command::new("tmux")
        .args(args)
        .output()
        .context("Failed to run tmux display-message")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("tmux display-message failed: {}", stderr);
    }

    let pane_id = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in pane ID")?
        .trim()
        .to_string();

    Ok(pane_id)
}

/// Execute a layout node, returning the pane IDs created
fn execute_node(node: &Layout, target_pane: &str) -> Result<Vec<String>> {
    match node {
        Layout::Pane => Ok(vec![target_pane.to_string()]),
        Layout::Split {
            direction,
            children,
        } => {
            if children.is_empty() {
                return Ok(vec![target_pane.to_string()]);
            }

            if children.len() == 1 {
                // Only one child, just recurse into it
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
                split_target = new_pane_id.clone();
                pane_ids.push(new_pane_id);
            }

            // Now recurse into each child with its corresponding pane
            let mut all_created_panes = Vec::new();
            for (i, (_, child)) in children.iter().enumerate() {
                let child_panes = execute_node(child, &pane_ids[i])?;
                all_created_panes.extend(child_panes);
            }

            Ok(all_created_panes)
        }
    }
}

/// Split a pane and return the new pane's ID
fn split_pane(target_pane: &str, dir_flag: &str, percentage: u8) -> Result<String> {
    let pct = percentage.to_string();
    let args = [
        "split-window",
        dir_flag,
        "-p",
        &pct,
        "-t",
        target_pane,
        "-P",
        "-F",
        "#{pane_id}",
    ];
    debug_log(&args);
    let output = Command::new("tmux")
        .args(args)
        .output()
        .context("Failed to run tmux split-window")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("tmux split-window failed: {}", stderr);
    }

    let new_pane_id = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in new pane ID")?
        .trim()
        .to_string();

    Ok(new_pane_id)
}
