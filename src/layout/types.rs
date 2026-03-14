//! Core layout types: Direction and Layout tree.

/// Split direction for panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Horizontal split (-h): panes arranged left/right
    Horizontal,
    /// Vertical split (-v): panes arranged top/bottom
    Vertical,
}

/// A layout tree node representing either a single pane or a split container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layout {
    /// A terminal pane (leaf node)
    Pane,
    /// A split container with weighted children
    Split {
        direction: Direction,
        /// (weight, child) pairs - weights are relative to siblings
        children: Vec<(u32, Layout)>,
    },
}

/// Convert fractional weights to the series of -p values tmux needs.
///
/// tmux's -p is "percentage of the pane being split", not of the whole window.
///
/// For [1, 1, 1, 1] (total 4):
///   - Split 1: new pane gets 3/4 (75%), first child keeps 1/4
///   - Split 2: of the 3/4, new pane gets 2/3 (67%), second child keeps 1/3
///   - Split 3: of the remaining 2/3 of 3/4, new pane gets 1/2 (50%)
///
/// So percentages for [1,1,1,1] are [75, 67, 50].
pub fn weights_to_split_percentages(weights: &[u32]) -> Vec<u8> {
    if weights.len() <= 1 {
        return vec![];
    }

    let total: u32 = weights.iter().sum();
    let mut result = Vec::with_capacity(weights.len() - 1);
    let mut remaining = total;

    for (i, _) in weights.iter().enumerate().skip(1) {
        remaining -= weights[i - 1];

        let sum_for_new_pane: u32 = weights[i..].iter().sum();
        let sum_remaining_before = remaining + weights[i - 1];
        let pct = ((sum_for_new_pane * 100) + (sum_remaining_before / 2)) / sum_remaining_before;
        result.push(pct as u8);
    }

    result
}

/// Create a grid layout with equal-sized panes.
///
/// # Examples
///
/// - `grid(2, 3)` creates a 2x3 grid (2 rows, 3 columns)
/// - `grid(1, 1)` returns a single `Layout::Pane`
pub fn grid(rows: u8, cols: u8) -> Layout {
    if rows == 1 && cols == 1 {
        return Layout::Pane;
    }

    if rows == 1 {
        // Single row, multiple columns
        return Layout::Split {
            direction: Direction::Horizontal,
            children: (0..cols).map(|_| (1, Layout::Pane)).collect(),
        };
    }

    if cols == 1 {
        // Single column, multiple rows
        return Layout::Split {
            direction: Direction::Vertical,
            children: (0..rows).map(|_| (1, Layout::Pane)).collect(),
        };
    }

    // Multiple rows and columns: rows of columns
    Layout::Split {
        direction: Direction::Vertical,
        children: (0..rows)
            .map(|_| {
                (
                    1,
                    Layout::Split {
                        direction: Direction::Horizontal,
                        children: (0..cols).map(|_| (1, Layout::Pane)).collect(),
                    },
                )
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weights_to_split_percentages_two_equal() {
        assert_eq!(weights_to_split_percentages(&[1, 1]), vec![50]);
    }

    #[test]
    fn test_weights_to_split_percentages_two_unequal() {
        assert_eq!(weights_to_split_percentages(&[2, 1]), vec![33]);
    }

    #[test]
    fn test_weights_to_split_percentages_three() {
        assert_eq!(weights_to_split_percentages(&[1, 2, 1]), vec![75, 33]);
    }

    #[test]
    fn test_weights_to_split_percentages_four_equal() {
        assert_eq!(
            weights_to_split_percentages(&[1, 1, 1, 1]),
            vec![75, 67, 50]
        );
    }

    #[test]
    fn test_weights_to_split_percentages_single() {
        let expected: Vec<u8> = vec![];
        assert_eq!(weights_to_split_percentages(&[1]), expected);
    }

    #[test]
    fn test_weights_to_split_percentages_empty() {
        let expected: Vec<u8> = vec![];
        assert_eq!(weights_to_split_percentages(&[]), expected);
    }

    #[test]
    fn test_grid_1x1() {
        assert_eq!(grid(1, 1), Layout::Pane);
    }

    #[test]
    fn test_grid_1x3() {
        assert_eq!(
            grid(1, 3),
            Layout::Split {
                direction: Direction::Horizontal,
                children: vec![(1, Layout::Pane), (1, Layout::Pane), (1, Layout::Pane)],
            }
        );
    }

    #[test]
    fn test_grid_2x1() {
        assert_eq!(
            grid(2, 1),
            Layout::Split {
                direction: Direction::Vertical,
                children: vec![(1, Layout::Pane), (1, Layout::Pane)],
            }
        );
    }

    #[test]
    fn test_grid_2x2() {
        assert_eq!(
            grid(2, 2),
            Layout::Split {
                direction: Direction::Vertical,
                children: vec![
                    (
                        1,
                        Layout::Split {
                            direction: Direction::Horizontal,
                            children: vec![(1, Layout::Pane), (1, Layout::Pane)],
                        }
                    ),
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
}
