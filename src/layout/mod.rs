//! Layout module: tree structure, DSL parser, and file loading.
//!
//! This module provides:
//! - Core types: [`Direction`], [`Layout`]
//! - DSL parsing: [`parse`] for strings like `"v(2,h(1,1))"`
//! - File loading: [`from_file`], [`from_yaml`], [`from_json`]
//! - Grid creation: [`grid`]
//! - Weight conversion: [`weights_to_split_percentages`]

mod file;
mod lexer;
mod parser;
mod types;

// Re-export public API
#[allow(unused_imports)] // Exported for library API, used in tests
pub use file::{from_file, from_json, from_yaml};
pub use parser::parse;
pub use types::{Direction, Layout, grid, weights_to_split_percentages};
