use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Horizontal, // -h (left/right)
    Vertical,   // -v (top/bottom)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layout {
    Pane,
    Split {
        direction: Direction,
        children: Vec<(u32, Layout)>, // (weight, child) pairs
    },
}

// ============================================================================
// Serde-compatible types for file-based layouts (YAML/JSON)
// ============================================================================

/// A serde-compatible layout node for file input
///
/// Example YAML:
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
    Pane {
        #[serde(default = "default_weight")]
        weight: u32,
        pane: bool,
    },
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
    /// Convert to the internal Layout type
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

    /// Get the weight of this layout node
    fn weight(&self) -> u32 {
        match self {
            FileLayout::Pane { weight, .. } => *weight,
            FileLayout::Split { weight, .. } => *weight,
        }
    }
}

/// Load a layout from a YAML string
pub fn from_yaml(yaml: &str) -> Result<Layout> {
    let file_layout: FileLayout =
        serde_yaml::from_str(yaml).map_err(|e| anyhow::anyhow!("Failed to parse YAML: {}", e))?;
    Ok(file_layout.into_layout())
}

/// Load a layout from a JSON string
pub fn from_json(json: &str) -> Result<Layout> {
    let file_layout: FileLayout =
        serde_json::from_str(json).map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;
    Ok(file_layout.into_layout())
}

/// Load a layout from a file (auto-detects YAML or JSON based on extension)
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

/// Convert fractional weights to the series of -p values tmux needs.
/// tmux's -p is "percentage of the pane being split", not of the whole window.
///
/// Example: weights [2, 1, 1] (total 4)
///   - First child keeps the original pane
///   - Second child: splits off from pane with 2 remaining parts, needs 1/2 = 50%
///   - Third child: splits off from pane with 1 remaining part, needs 1/1 = 100%... wait, that's wrong
///
/// Actually, we need to think of it differently:
///   - We start with the full pane
///   - First child takes weights[0] / total of the space (implicitly, by keeping the pane)
///   - Each subsequent split takes weights[i] / remaining_total of what's left
///
/// For [2, 1, 1] (total 4):
///   - First child gets 2/4 = 50% (keeps the pane)
///   - Second split: remaining = 2 (1+1), second child wants 1/2 = 50% of remainder → -p 50
///   - Third split: remaining = 1, third child wants 1/1 = 100% of remainder → -p 100? No...
///
/// The issue is that -p specifies what the NEW pane gets, not what remains.
/// So for the last split, -p 50 means the new pane gets 50% of the current pane.
///
/// Let me reconsider with [1, 1, 1, 1] (total 4):
///   - Start with full pane (will be split to give first child 1/4)
///   - Split 1: new pane gets 3/4 (75%), first child keeps 1/4
///   - Split 2: of the 3/4, new pane gets 2/3 (67%), second child keeps 1/3 of the 3/4
///   - Split 3: of the remaining 2/3 of 3/4, new pane gets 1/2 (50%), third child keeps half
///
/// So percentages for [1,1,1,1] are [75, 67, 50] - matches the spec!
pub fn weights_to_split_percentages(weights: &[u32]) -> Vec<u8> {
    if weights.len() <= 1 {
        return vec![];
    }

    let total: u32 = weights.iter().sum();
    let mut result = Vec::with_capacity(weights.len() - 1);
    let mut remaining = total;

    for (i, _) in weights.iter().enumerate().skip(1) {
        // After allocating space for children 0..i, how much is left?
        remaining -= weights[i - 1];

        // The new pane (containing children i..n) gets (sum of weights i..n) / remaining
        // But wait, we're splitting off everything AFTER the current child
        // Actually: remaining is sum of weights[i-1..n]
        // After this split, the new pane gets sum of weights[i..n]
        // So percentage = sum(weights[i..n]) / remaining = (remaining - weights[i-1]) / remaining
        // But we already subtracted weights[i-1], so percentage = remaining / old_remaining

        // Hmm, let me re-derive this more carefully.
        // Before split i (1-indexed): we have a pane representing weights[i-1..n]
        // We want to split it so:
        //   - weights[i-1] stays in the original pane
        //   - weights[i..n] goes to the new pane
        //
        // sum_remaining_before = weights[i-1..n].sum()
        // sum_for_new_pane = weights[i..n].sum()
        // percentage = sum_for_new_pane / sum_remaining_before * 100

        let sum_for_new_pane: u32 = weights[i..].iter().sum();
        let sum_remaining_before = remaining + weights[i - 1];
        let pct = ((sum_for_new_pane * 100) + (sum_remaining_before / 2)) / sum_remaining_before;
        result.push(pct as u8);
    }

    result
}

/// Create a grid layout with equal-sized panes
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

// ============================================================================
// DSL Parser
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    V,
    H,
    LParen,
    RParen,
    Comma,
    Number(u32),
}

struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek_char() {
            self.pos += c.len_utf8();
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>> {
        self.skip_whitespace();

        let c = match self.peek_char() {
            Some(c) => c,
            None => return Ok(None),
        };

        let token = match c {
            'v' | 'V' => {
                self.advance();
                Token::V
            }
            'h' | 'H' => {
                self.advance();
                Token::H
            }
            '(' => {
                self.advance();
                Token::LParen
            }
            ')' => {
                self.advance();
                Token::RParen
            }
            ',' => {
                self.advance();
                Token::Comma
            }
            '0'..='9' => {
                let start = self.pos;
                while let Some(c) = self.peek_char() {
                    if c.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let num_str = &self.input[start..self.pos];
                let num: u32 = num_str.parse().map_err(|_| {
                    anyhow::anyhow!("Invalid number '{}' at position {}", num_str, start)
                })?;
                Token::Number(num)
            }
            _ => bail!("Unexpected character '{}' at position {}", c, self.pos),
        };

        Ok(Some(token))
    }

    fn position(&self) -> usize {
        self.pos
    }
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Option<Token>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Result<Self> {
        let mut lexer = Lexer::new(input);
        let current = lexer.next_token()?;
        Ok(Self { lexer, current })
    }

    fn advance(&mut self) -> Result<()> {
        self.current = self.lexer.next_token()?;
        Ok(())
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        match &self.current {
            Some(token) if *token == expected => {
                self.advance()?;
                Ok(())
            }
            Some(token) => bail!(
                "Expected {:?} but found {:?} at position {}",
                expected,
                token,
                self.lexer.position()
            ),
            None => bail!("Expected {:?} but reached end of input", expected),
        }
    }

    /// Parse the full layout
    /// layout := 'v' '(' entries ')' | 'h' '(' entries ')' | number
    fn parse_layout(&mut self) -> Result<(u32, Layout)> {
        match &self.current {
            Some(Token::V) => {
                self.advance()?;
                self.expect(Token::LParen)?;
                let children = self.parse_entries()?;
                self.expect(Token::RParen)?;
                Ok((
                    1,
                    Layout::Split {
                        direction: Direction::Vertical,
                        children,
                    },
                ))
            }
            Some(Token::H) => {
                self.advance()?;
                self.expect(Token::LParen)?;
                let children = self.parse_entries()?;
                self.expect(Token::RParen)?;
                Ok((
                    1,
                    Layout::Split {
                        direction: Direction::Horizontal,
                        children,
                    },
                ))
            }
            Some(Token::Number(n)) => {
                let weight = *n;
                self.advance()?;
                Ok((weight, Layout::Pane))
            }
            Some(token) => bail!(
                "Expected 'v', 'h', or number but found {:?} at position {}",
                token,
                self.lexer.position()
            ),
            None => bail!("Unexpected end of input"),
        }
    }

    /// Parse comma-separated entries
    /// entries := layout (',' layout)*
    fn parse_entries(&mut self) -> Result<Vec<(u32, Layout)>> {
        let mut entries = vec![self.parse_layout()?];

        while let Some(Token::Comma) = &self.current {
            self.advance()?;
            entries.push(self.parse_layout()?);
        }

        Ok(entries)
    }
}

/// Parse a layout DSL string into a Layout tree
///
/// Grammar:
///   layout  := 'v' '(' entries ')' | 'h' '(' entries ')' | number
///   entries := layout (',' layout)*
///   number  := bare number → Pane with associated weight
///
/// Examples:
///   "v(2,1)"       → vertical split, top gets 2/3, bottom gets 1/3
///   "h(1,1,1)"     → horizontal split, 3 equal columns
///   "v(2,h(1,1))"  → top gets 2/3, bottom is split horizontally
pub fn parse(input: &str) -> Result<Layout> {
    let mut parser = Parser::new(input)?;
    let (_, layout) = parser.parse_layout()?;

    // Ensure we consumed all input
    if parser.current.is_some() {
        bail!(
            "Unexpected token {:?} at position {}",
            parser.current,
            parser.lexer.position()
        );
    }

    Ok(layout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weights_to_split_percentages_two_equal() {
        // [1, 1] → first keeps 50%, second gets 50%
        assert_eq!(weights_to_split_percentages(&[1, 1]), vec![50]);
    }

    #[test]
    fn test_weights_to_split_percentages_two_unequal() {
        // [2, 1] → first keeps 67%, second gets 33%
        assert_eq!(weights_to_split_percentages(&[2, 1]), vec![33]);
    }

    #[test]
    fn test_weights_to_split_percentages_three() {
        // [1, 2, 1] → total 4
        // First child: 1/4
        // Second split: remaining is 3 (2+1), new pane (for children 1,2) gets 3/4 = 75%
        // Third split: remaining is 1, new pane gets 1/3 of the 3/4... wait
        // Let me recalculate:
        // After first split (75%), we have: first child (25%), and new pane (75%)
        // In the new pane (representing weights 2,1), we split:
        //   - Second child wants 2/3 of it
        //   - Third child wants 1/3 of it
        //   - So we split off 1/3 = 33% for the third child
        // Result: [75, 33]
        assert_eq!(weights_to_split_percentages(&[1, 2, 1]), vec![75, 33]);
    }

    #[test]
    fn test_weights_to_split_percentages_four_equal() {
        // [1, 1, 1, 1] → [75, 67, 50] per spec
        assert_eq!(
            weights_to_split_percentages(&[1, 1, 1, 1]),
            vec![75, 67, 50]
        );
    }

    #[test]
    fn test_weights_to_split_percentages_single() {
        // [1] → no splits needed
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

    #[test]
    fn test_parse_simple_vertical() {
        let layout = parse("v(2,1)").unwrap();
        assert_eq!(
            layout,
            Layout::Split {
                direction: Direction::Vertical,
                children: vec![(2, Layout::Pane), (1, Layout::Pane)],
            }
        );
    }

    #[test]
    fn test_parse_simple_horizontal() {
        let layout = parse("h(1,1,1)").unwrap();
        assert_eq!(
            layout,
            Layout::Split {
                direction: Direction::Horizontal,
                children: vec![(1, Layout::Pane), (1, Layout::Pane), (1, Layout::Pane)],
            }
        );
    }

    #[test]
    fn test_parse_nested() {
        let layout = parse("v(2,h(1,1))").unwrap();
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
    fn test_parse_deeply_nested() {
        let layout = parse("v(1,v(1,1))").unwrap();
        assert_eq!(
            layout,
            Layout::Split {
                direction: Direction::Vertical,
                children: vec![
                    (1, Layout::Pane),
                    (
                        1,
                        Layout::Split {
                            direction: Direction::Vertical,
                            children: vec![(1, Layout::Pane), (1, Layout::Pane)],
                        }
                    ),
                ],
            }
        );
    }

    #[test]
    fn test_parse_with_whitespace() {
        let layout = parse("v( 2 , h( 1 , 1 ) )").unwrap();
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
    fn test_parse_error_missing_paren() {
        assert!(parse("v(2,h(1,1)").is_err());
    }

    #[test]
    fn test_parse_error_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn test_parse_error_invalid_char() {
        assert!(parse("x(1,1)").is_err());
    }

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
        // Weight defaults to 1 when not specified
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
