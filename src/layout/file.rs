//! File-based layout loading (YAML/JSON).

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::types::{Direction, Layout};

/// A serde-compatible layout node for file input.
///
/// # Example YAML
///
/// ```yaml
/// direction: vertical
/// children:
///   - weight: 2
///     pane: true
///   - weight: 1
///     direction: horizontal
///     children:
///       - weight: 1
///         pane: true
///       - weight: 1
///         pane: true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileLayout {
    /// A terminal pane node
    Pane {
        #[serde(default = "default_weight")]
        weight: u32,
        pane: bool,
    },
    /// A split container node
    Split {
        #[serde(default = "default_weight")]
        weight: u32,
        direction: FileDirection,
        children: Vec<FileLayout>,
    },
}

fn default_weight() -> u32 {
    1
}

/// Direction in file format (lowercase serialization).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileDirection {
    Horizontal,
    Vertical,
}

impl From<FileDirection> for Direction {
    fn from(fd: FileDirection) -> Self {
        match fd {
            FileDirection::Horizontal => Direction::Horizontal,
            FileDirection::Vertical => Direction::Vertical,
        }
    }
}

impl From<Direction> for FileDirection {
    fn from(d: Direction) -> Self {
        match d {
            Direction::Horizontal => FileDirection::Horizontal,
            Direction::Vertical => FileDirection::Vertical,
        }
    }
}

impl FileLayout {
    /// Convert to the internal Layout type.
    pub fn into_layout(self) -> Layout {
        match self {
            FileLayout::Pane { .. } => Layout::Pane,
            FileLayout::Split {
                direction,
                children,
                ..
            } => Layout::Split {
                direction: direction.into(),
                children: children
                    .into_iter()
                    .map(|child| {
                        let weight = child.weight();
                        (weight, child.into_layout())
                    })
                    .collect(),
            },
        }
    }

    /// Get the weight of this layout node.
    fn weight(&self) -> u32 {
        match self {
            FileLayout::Pane { weight, .. } => *weight,
            FileLayout::Split { weight, .. } => *weight,
        }
    }
}

/// Load a layout from a YAML string.
pub fn from_yaml(yaml: &str) -> Result<Layout> {
    let file_layout: FileLayout =
        serde_yaml::from_str(yaml).map_err(|e| anyhow::anyhow!("Failed to parse YAML: {}", e))?;
    Ok(file_layout.into_layout())
}

/// Load a layout from a JSON string.
pub fn from_json(json: &str) -> Result<Layout> {
    let file_layout: FileLayout =
        serde_json::from_str(json).map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;
    Ok(file_layout.into_layout())
}

/// Load a layout from a file (auto-detects YAML or JSON based on extension).
pub fn from_file(path: &std::path::Path) -> Result<Layout> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", path.display(), e))?;

    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension.to_lowercase().as_str() {
        "json" => from_json(&content),
        "yaml" | "yml" => from_yaml(&content),
        _ => {
            // Try YAML first (it's a superset of JSON), then JSON
            from_yaml(&content).or_else(|_| from_json(&content))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_yaml_simple() {
        let yaml = r#"
direction: vertical
children:
  - weight: 2
    pane: true
  - weight: 1
    pane: true
"#;
        let layout = from_yaml(yaml).unwrap();
        assert_eq!(
            layout,
            Layout::Split {
                direction: Direction::Vertical,
                children: vec![(2, Layout::Pane), (1, Layout::Pane)],
            }
        );
    }

    #[test]
    fn test_from_yaml_nested() {
        let yaml = r#"
direction: vertical
children:
  - weight: 2
    pane: true
  - weight: 1
    direction: horizontal
    children:
      - weight: 1
        pane: true
      - weight: 1
        pane: true
"#;
        let layout = from_yaml(yaml).unwrap();
        assert_eq!(
            layout,
            Layout::Split {
                direction: Direction::Vertical,
                children: vec![
                    (2, Layout::Pane),
                    (
                        1,
                        Layout::Split {
                            direction: Direction::Horizontal,
                            children: vec![(1, Layout::Pane), (1, Layout::Pane)],
                        }
                    ),
                ],
            }
        );
    }

    #[test]
    fn test_from_json_simple() {
        let json = r#"{
            "direction": "horizontal",
            "children": [
                {"weight": 1, "pane": true},
                {"weight": 1, "pane": true}
            ]
        }"#;
        let layout = from_json(json).unwrap();
        assert_eq!(
            layout,
            Layout::Split {
                direction: Direction::Horizontal,
                children: vec![(1, Layout::Pane), (1, Layout::Pane)],
            }
        );
    }

    #[test]
    fn test_from_yaml_default_weight() {
        let yaml = r#"
direction: vertical
children:
  - pane: true
  - pane: true
"#;
        let layout = from_yaml(yaml).unwrap();
        assert_eq!(
            layout,
            Layout::Split {
                direction: Direction::Vertical,
                children: vec![(1, Layout::Pane), (1, Layout::Pane)],
            }
        );
    }
}
