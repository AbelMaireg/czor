//! Recursive descent parser for the layout DSL.

use anyhow::{Result, bail};

use super::lexer::{Lexer, Token};
use super::types::{Direction, Layout};

/// A recursive descent parser for layout DSL strings.
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Option<Token>,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given input string.
    pub fn new(input: &'a str) -> Result<Self> {
        let mut lexer = Lexer::new(input);
        let current = lexer.next_token()?;
        Ok(Self { lexer, current })
    }

    /// Advance to the next token.
    fn advance(&mut self) -> Result<()> {
        self.current = self.lexer.next_token()?;
        Ok(())
    }

    /// Expect a specific token and advance.
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

    /// Parse a layout node with its weight.
    ///
    /// Grammar:
    ///   layout := 'v' '(' entries ')' | 'h' '(' entries ')' | number
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

    /// Parse comma-separated layout entries.
    ///
    /// Grammar:
    ///   entries := layout (',' layout)*
    fn parse_entries(&mut self) -> Result<Vec<(u32, Layout)>> {
        let mut entries = vec![self.parse_layout()?];

        while let Some(Token::Comma) = &self.current {
            self.advance()?;
            entries.push(self.parse_layout()?);
        }

        Ok(entries)
    }

    /// Check if there are remaining tokens.
    pub fn has_remaining(&self) -> bool {
        self.current.is_some()
    }

    /// Get the current token for error reporting.
    pub fn current_token(&self) -> Option<&Token> {
        self.current.as_ref()
    }

    /// Get the current lexer position.
    pub fn position(&self) -> usize {
        self.lexer.position()
    }
}

/// Parse a layout DSL string into a Layout tree.
///
/// # Grammar
///
/// ```text
/// layout  := 'v' '(' entries ')' | 'h' '(' entries ')' | number
/// entries := layout (',' layout)*
/// number  := bare number → Pane with associated weight
/// ```
///
/// # Examples
///
/// - `"v(2,1)"` → vertical split, top gets 2/3, bottom gets 1/3
/// - `"h(1,1,1)"` → horizontal split, 3 equal columns
/// - `"v(2,h(1,1))"` → top gets 2/3, bottom is split horizontally
pub fn parse(input: &str) -> Result<Layout> {
    let mut parser = Parser::new(input)?;
    let (_, layout) = parser.parse_layout()?;

    // Ensure we consumed all input
    if parser.has_remaining() {
        bail!(
            "Unexpected token {:?} at position {}",
            parser.current_token(),
            parser.position()
        );
    }

    Ok(layout)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
