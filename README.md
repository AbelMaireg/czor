# czor

[![CI](https://github.com/AbelMaireg/czor/actions/workflows/ci.yml/badge.svg)](https://github.com/AbelMaireg/czor/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/czor.svg)](https://crates.io/crates/czor)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A tmux layout manager that makes it easy to create and apply complex pane layouts.

## Features

- **Split panes with ratios** - Divide panes using simple ratio notation (e.g., `2:1`, `1:1:1`)
- **Layout DSL** - Define complex nested layouts with a simple DSL (e.g., `v(2,h(1,1))`)
- **Grid layouts** - Quickly create grid arrangements (e.g., `2x3` for 2 rows, 3 columns)
- **File-based layouts** - Load layouts from YAML or JSON files

## Installation

### From source

```bash
cargo install --path .
```

### From crates.io

```bash
cargo install czor
```

## Usage

All commands must be run from within a tmux session.

### Split panes with a ratio

```bash
# Split vertically with 2:1 ratio
czor split v 2:1

# Split horizontally into 3 equal parts
czor split h 1:1:1
```

### Apply a layout DSL

```bash
# Vertical split: top pane (weight 2), bottom split horizontally (1:1)
czor layout "v(2,h(1,1))"
```

### Create a grid

```bash
# Create a 2x3 grid (2 rows, 3 columns)
czor grid 2x3
```

### Apply layout from file

```bash
czor apply layout.yaml
```

#### YAML layout example

```yaml
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
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release history.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
