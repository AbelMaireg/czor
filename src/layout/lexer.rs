//! Lexer for the layout DSL.

use anyhow::{bail, Result};

/// Tokens produced by the lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Vertical split indicator
    V,
    /// Horizontal split indicator
    H,
    /// Left parenthesis '('
    LParen,
    /// Right parenthesis ')'
    RParen,
    /// Comma separator ','
    Comma,
    /// Numeric weight value
    Number(u32),
}

/// A lexer that tokenizes layout DSL input strings.
pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input string.
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    /// Peek at the next character without consuming it.
    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Advance the position by one character.
    fn advance(&mut self) {
        if let Some(c) = self.peek_char() {
            self.pos += c.len_utf8();
        }
    }

    /// Skip whitespace characters.
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Get the next token from the input.
    pub fn next_token(&mut self) -> Result<Option<Token>> {
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

    /// Get the current position in the input.
    pub fn position(&self) -> usize {
        self.pos
    }
}
