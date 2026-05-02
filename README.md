# czor

[![CI](https://github.com/AbelMaireg/czor/actions/workflows/ci.yml/badge.svg)](https://github.com/AbelMaireg/czor/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/czor.svg)](https://crates.io/crates/czor)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A tmux layout manager. Describe a pane arrangement once — as a ratio, a compact DSL expression, a grid shorthand, or a YAML/JSON file — and czor splits your window into it instantly.

---

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Commands](#commands)
  - [split](#split)
  - [layout](#layout)
  - [grid](#grid)
  - [apply](#apply)
- [Layout DSL Reference](#layout-dsl-reference)
- [File Format Reference](#file-format-reference)
- [Global Flags](#global-flags)
- [Common Patterns](#common-patterns)
- [Changelog](#changelog)
- [Contributing](#contributing)
- [License](#license)

---

## Installation

czor must be run from within a tmux session.

### From crates.io

```bash
cargo install czor
```

### From source

```bash
git clone https://github.com/AbelMaireg/czor.git
cd czor
cargo install --path .
```

---

## Quick Start

```bash
# Three equal columns
czor split h 1:1:1

# Top pane (two-thirds) + two columns below
czor layout "v(2,h(1,1))"

# 2×3 grid (2 rows, 3 columns)
czor grid 2x3

# Load a saved layout
czor apply myproject.yaml
```

---

## Commands

### split

Divide the current pane along one axis using a weight ratio.

```
czor split <direction> <ratio>
```

| Argument | Values | Description |
|----------|--------|-------------|
| `direction` | `v` / `vertical` | Stack panes top-to-bottom |
| | `h` / `horizontal` | Place panes left-to-right |
| `ratio` | `W:W:…` | Colon-separated integer weights |

**Examples**

```bash
czor split v 2:1
```
```
+------------------+
|                  |
|   pane 1  (2/3)  |
|                  |
+------------------+
|   pane 2  (1/3)  |
+------------------+
```

```bash
czor split h 1:1:1
```
```
+--------+--------+--------+
|        |        |        |
| pane 1 | pane 2 | pane 3 |
|        |        |        |
+--------+--------+--------+
```

Weights are proportional, not percentages. `1:1` and `3:3` produce the same result.

---

### layout

Apply an arbitrarily nested layout described by the [DSL](#layout-dsl-reference).

```
czor layout "<dsl>"
```

**Examples**

```bash
czor layout "v(2,h(1,1))"
```
```
+------------------+
|                  |
|   pane 1  (2/3)  |
|                  |
+---------+--------+
| pane 2  | pane 3 |
+---------+--------+
```

```bash
czor layout "h(1,v(1,1),1)"
```
```
+--------+---------+--------+
|        | pane 2  |        |
| pane 1 +---------+ pane 4 |
|        | pane 3  |        |
+--------+---------+--------+
```

---

### grid

Create a uniform grid of panes.

```
czor grid <RxC>
```

`R` is the number of rows, `C` is the number of columns.

**Examples**

```bash
czor grid 2x3
```
```
+--------+--------+--------+
| pane 1 | pane 2 | pane 3 |
+--------+--------+--------+
| pane 4 | pane 5 | pane 6 |
+--------+--------+--------+
```

```bash
czor grid 1x4    # four equal columns
czor grid 3x1    # three equal rows
czor grid 1x1    # no-op (single pane)
```

---

### apply

Load a layout from a YAML or JSON file. The file extension determines the parser
(`yaml` / `yml` → YAML, `json` → JSON). Files with no recognized extension are
tried as YAML first, then JSON.

```
czor apply <path>
```

**Example**

```bash
czor apply ~/.config/czor/dev.yaml
czor apply layouts/dashboard.json
```

See [File Format Reference](#file-format-reference) for the schema.

---

## Layout DSL Reference

The DSL is the most expressive input format. It encodes the full layout tree in a
single compact string, making it easy to embed in shell aliases or scripts.

### Grammar

```
layout  :=  'v' '(' entries ')'   # vertical split (top-to-bottom)
          | 'h' '(' entries ')'   # horizontal split (left-to-right)
          | number                # leaf pane with given weight
entries :=  layout (',' layout)*
```

- `v(…)` stacks children top-to-bottom.
- `h(…)` places children left-to-right.
- A bare number is a leaf pane; the number is its weight relative to siblings.
- Whitespace around tokens is allowed: `v( 2 , h( 1 , 1 ) )` is valid.

### Examples

| Expression | Result |
|---|---|
| `v(1,1)` | Two equal rows |
| `h(1,1)` | Two equal columns |
| `v(2,1)` | Top row twice the height of bottom row |
| `h(3,1)` | Left column three times the width of right |
| `v(2,h(1,1))` | Top pane (2/3) + two equal columns below |
| `h(1,v(1,1),1)` | Left + right panes flanking a vertically-split center |
| `v(1,h(1,1,1),1)` | Three rows; middle row has three equal columns |
| `h(1,v(1,v(1,1)))` | Left pane + right side split into three rows |

### Nesting depth

There is no hard limit. `v(1,v(1,v(1,1)))` creates four rows where each
successive row is half the remaining space.

---

## File Format Reference

File layouts are YAML or JSON. Both formats share the same schema.

### Schema

A node is either a **pane** (leaf) or a **split** (container):

```
node := pane | split

pane:
  weight:     <integer>   # optional, default 1
  pane:       true

split:
  weight:     <integer>   # optional, default 1
  direction:  vertical | horizontal
  children:   [node, ...]
```

The top-level node must be a `split`.

### YAML example

```yaml
direction: vertical
children:
  - weight: 2
    pane: true
  - weight: 1
    direction: horizontal
    children:
      - pane: true        # weight defaults to 1
      - pane: true
```

Equivalent to `czor layout "v(2,h(1,1))"`.

### JSON example

```json
{
  "direction": "horizontal",
  "children": [
    { "weight": 1, "pane": true },
    {
      "weight": 2,
      "direction": "vertical",
      "children": [
        { "pane": true },
        { "pane": true }
      ]
    },
    { "weight": 1, "pane": true }
  ]
}
```

Equivalent to `czor layout "h(1,v(1,1),1)"`.

---

## Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--debug` | `-d` | Print each `tmux` command to stderr before running it |
| `--help` | `-h` | Show help |
| `--version` | `-V` | Show version |

`--debug` is useful for understanding exactly what czor does or for diagnosing
unexpected layouts:

```bash
czor --debug layout "v(2,h(1,1))"
# + tmux display-message -p #{pane_id}
# + tmux split-window -v -p 33 -t %0 -P -F #{pane_id}
# + tmux split-window -h -p 50 -t %0 -P -F #{pane_id}
```

---

## Common Patterns

**Dev environment: editor + terminal + logs**

```bash
czor layout "v(3,h(1,1))"
# or save it:
cat > ~/.config/czor/dev.yaml <<'EOF'
direction: vertical
children:
  - weight: 3
    pane: true
  - weight: 1
    direction: horizontal
    children:
      - pane: true
      - pane: true
EOF
czor apply ~/.config/czor/dev.yaml
```

**Pair programming: equal side-by-side terminals**

```bash
czor split h 1:1
```

**Four-quadrant dashboard**

```bash
czor grid 2x2
```

**Wide monitor: main pane with narrow sidebar**

```bash
czor split h 3:1
```

**Shell alias for a project layout**

```bash
# In ~/.bashrc or ~/.config/fish/config.fish
alias devenv='czor layout "h(2,v(1,1))"'
```

---

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release history.

---

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
