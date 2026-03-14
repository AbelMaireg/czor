use anyhow::{bail, Context, Result};
use std::process::Command;

use crate::layout::{weights_to_split_percentages, Direction, Layout};

/// Execute a layout in the current tmux window
pub fn execute(layout: &Layout) -> Result<()> {
    // Get the current pane ID
    let current_pane = get_current_pane_id()?;
    execute_node(layout, &current_pane)?;
    Ok(())
}

/// Get the current pane ID
fn get_current_pane_id() -> Result<String> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "#{pane_id}"])
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

            // First child inherits the target pane
            // Subsequent children are created by splitting
            let mut pane_ids = vec![target_pane.to_string()];

            for pct in &percentages {
                // Split the target pane to create a new pane
                // The new pane gets `pct` percent of the current pane
                let new_pane_id = split_pane(target_pane, dir_flag, *pct)?;
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
    let output = Command::new("tmux")
        .args([
            "split-window",
            dir_flag,
            "-p",
            &percentage.to_string(),
            "-t",
            target_pane,
            "-P",
            "-F",
            "#{pane_id}",
        ])
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
