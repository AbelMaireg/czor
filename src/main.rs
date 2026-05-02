mod exec;
mod layout;

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};

use layout::{Direction, Layout};

#[derive(Parser)]
#[command(name = "czor")]
#[command(version)]
#[command(about = "A tmux layout manager", long_about = None)]
struct Cli {
    /// Print each tmux command before executing it
    #[arg(short, long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Split the current pane with a ratio
    Split {
        /// Direction: 'v' for vertical, 'h' for horizontal
        direction: String,
        /// Ratio string (e.g., "2:1", "1:1:1")
        ratio: String,
    },
    /// Apply a layout DSL string
    Layout {
        /// Layout DSL (e.g., "v(2,h(1,1))")
        dsl: String,
    },
    /// Create a grid of panes
    Grid {
        /// Grid dimensions (e.g., "2x3" for 2 rows, 3 columns)
        dimensions: String,
    },
    /// Apply a layout from a file (YAML or JSON)
    Apply {
        /// Path to the layout file
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    if std::env::var("TMUX").is_err() {
        bail!("Not inside a tmux session. Please run czor from within tmux.");
    }

    let cli = Cli::parse();

    exec::set_debug(cli.debug);

    match cli.command {
        Commands::Split { direction, ratio } => {
            let dir = parse_direction(&direction)?;
            let weights = parse_ratio(&ratio)?;
            let layout = Layout::Split {
                direction: dir,
                children: weights.into_iter().map(|w| (w, Layout::Pane)).collect(),
            };
            exec::execute(&layout)?;
        }
        Commands::Layout { dsl } => {
            let layout = layout::parse(&dsl)?;
            exec::execute(&layout)?;
        }
        Commands::Grid { dimensions } => {
            let (rows, cols) = parse_grid_dimensions(&dimensions)?;
            let layout = layout::grid(rows, cols);
            exec::execute(&layout)?;
        }
        Commands::Apply { file } => {
            let layout = layout::from_file(&file)?;
            exec::execute(&layout)?;
        }
    }

    Ok(())
}

fn parse_direction(s: &str) -> Result<Direction> {
    match s.to_lowercase().as_str() {
        "v" | "vertical" => Ok(Direction::Vertical),
        "h" | "horizontal" => Ok(Direction::Horizontal),
        _ => bail!("Invalid direction '{}'. Use 'v' or 'h'.", s),
    }
}

fn parse_ratio(s: &str) -> Result<Vec<u32>> {
    let parts: Result<Vec<u32>, _> = s.split(':').map(|p| p.trim().parse::<u32>()).collect();

    match parts {
        Ok(weights) => {
            if weights.is_empty() {
                bail!("Ratio cannot be empty");
            }
            if weights.contains(&0) {
                bail!("Ratio weights must be greater than 0");
            }
            Ok(weights)
        }
        Err(_) => bail!(
            "Invalid ratio format '{}'. Use format like '2:1' or '1:1:1'.",
            s
        ),
    }
}

fn parse_grid_dimensions(s: &str) -> Result<(u8, u8)> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        bail!("Invalid grid format '{}'. Use format like '2x3'.", s);
    }
    let rows: u8 = parts[0]
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid row count in '{}'", s))?;
    let cols: u8 = parts[1]
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid column count in '{}'", s))?;
    if rows == 0 || cols == 0 {
        bail!("Grid dimensions must be at least 1x1");
    }
    Ok((rows, cols))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ratio() {
        assert_eq!(parse_ratio("2:1").unwrap(), vec![2, 1]);
        assert_eq!(parse_ratio("1:1:1").unwrap(), vec![1, 1, 1]);
        assert_eq!(parse_ratio("1:2:1").unwrap(), vec![1, 2, 1]);
        assert!(parse_ratio("").is_err());
        assert!(parse_ratio("0:1").is_err());
        assert!(parse_ratio("a:b").is_err());
    }

    #[test]
    fn test_parse_direction() {
        assert!(matches!(parse_direction("v").unwrap(), Direction::Vertical));
        assert!(matches!(parse_direction("V").unwrap(), Direction::Vertical));
        assert!(matches!(
            parse_direction("vertical").unwrap(),
            Direction::Vertical
        ));
        assert!(matches!(
            parse_direction("h").unwrap(),
            Direction::Horizontal
        ));
        assert!(matches!(
            parse_direction("H").unwrap(),
            Direction::Horizontal
        ));
        assert!(matches!(
            parse_direction("horizontal").unwrap(),
            Direction::Horizontal
        ));
        assert!(parse_direction("x").is_err());
    }

    #[test]
    fn test_parse_grid_dimensions() {
        assert_eq!(parse_grid_dimensions("2x3").unwrap(), (2, 3));
        assert_eq!(parse_grid_dimensions("1x1").unwrap(), (1, 1));
        assert!(parse_grid_dimensions("0x1").is_err());
        assert!(parse_grid_dimensions("2").is_err());
        assert!(parse_grid_dimensions("axb").is_err());
    }
}
