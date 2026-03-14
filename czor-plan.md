# czor — Development Plan

## Architecture

```
CLI Input → Parser → Layout Tree → Executor → tmux
```

Every phase keeps this pipeline intact. The tree is the central data structure — everything flows through it.

---

## Module Map

Start with **3 files**, not 6. Split only when a module earns its own file.

```
czor/
└── src/
    ├── main.rs        # CLI (clap) + entrypoint
    ├── layout.rs      # Layout tree + parser + Display impl
    └── exec.rs        # Tree walk → tmux commands (includes tmux wrapper)
```

**When to split further:**

| Trigger | Action |
|---|---|
| Parser exceeds ~150 lines | Extract `parser.rs` from `layout.rs` |
| You add JSON/YAML input | Add `serde` support in a `formats.rs` |
| You add preview/ASCII output | Add `preview.rs` |
| tmux wrapper grows beyond `run()` | Extract `tmux.rs` from `exec.rs` |

Don't pre-create files you aren't writing code in yet.

---

## Data Structures

These are stable from day one and shouldn't change much across phases.

```rust
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Horizontal, // -h  (left/right)
    Vertical,   // -v  (top/bottom)
}

#[derive(Debug)]
pub enum Layout {
    Pane,
    Split {
        direction: Direction,
        children: Vec<(u32, Layout)>, // (weight, child) pairs
    },
}
```

Key decisions:
- **Ratios live alongside children, not in a parallel vec.** Impossible to mismatch counts.
- **Ratios are fractional weights, not percentages.** `2:1` means "2 parts and 1 part" — the executor converts to percentages at split time. This means `2:1`, `4:2`, and `200:100` all produce the same layout. Users never have to think about summing to 100.

**Weight → percentage conversion** (needed when calling `tmux split-window -p`):

```rust
/// Convert fractional weights to the series of -p values tmux needs.
/// tmux's -p is "percentage of the pane being split", not of the whole window.
///
/// Example: weights [2, 1, 1] (total 4)
///   split 1: new pane gets 50% of whole   → -p 50  (splits off 2/4)
///   split 2: new pane gets 50% of remainder → -p 50  (splits off 1/2)
///
fn weights_to_percentages(weights: &[u32]) -> Vec<u8> {
    let mut remaining: u32 = weights.iter().sum();
    weights.iter().skip(1).map(|&w| {
        remaining -= weights[weights.iter().position(|x| *x == remaining - weights.iter().rev().take_while(|&&r| r != w).sum::<u32>()).unwrap_or(0)];
        // Simpler approach:
        let pct = (w * 100) / remaining;
        remaining -= w;
        pct as u8
    }).collect()
}
```

Actually, keep it simple — here's the clean version:

```rust
fn weights_to_split_percentages(weights: &[u32]) -> Vec<u8> {
    // First child keeps the pane. Each subsequent child splits off from the remainder.
    // We iterate in reverse: last child splits first from the bottom/right.
    let total: u32 = weights.iter().sum();
    let mut result = Vec::new();
    let mut remaining = total;

    for &w in weights.iter().skip(1) {
        remaining -= weights[result.len()]; // subtract the previous child's weight
        let pct = (w * 100 + remaining / 2) / remaining; // rounded
        result.push(pct as u8);
    }
    result
}
```

This is the trickiest math in the project. Write unit tests for it early — see the pitfalls section.

---

## Dependencies

```toml
[package]
name = "czor"
version = "0.0.0"  # placeholder — real version comes from git tags
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
anyhow = "1"
```

That's it to start. Add `thiserror` when you define custom error types. Add `nom` only if hand-rolled parsing becomes painful. Add `serde` only in Phase 3.

---

## Versioning — Rolling Release

Use **calendar-based rolling versions** derived from git tags. No manually bumped `Cargo.toml` version — the repo is the source of truth.

**Format:** `YYYY.MM.DD` (e.g. `2026.03.14`)

If you ship more than once in a day, append a patch counter: `2026.03.14.1`, `2026.03.14.2`.

**Workflow:**

1. Merge to `main` when a feature/fix is ready.
2. Tag the commit:
   ```bash
   git tag 2026.03.14
   git push --tags
   ```
3. CI builds the binary from the tag.

**Injecting the version at build time:**

Option A — **build script** (simplest, no extra deps):

```rust
// build.rs
fn main() {
    let version = std::process::Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "dev".to_string());
    println!("cargo:rustc-env=CZOR_VERSION={}", version.trim());
}
```

```rust
// main.rs
fn main() {
    let cli = Cli::parse();
    // ...
}

// clap will pick up the version:
#[derive(Parser)]
#[command(version = env!("CZOR_VERSION"))]
struct Cli { /* ... */ }
```

Now `czor --version` prints `czor 2026.03.14` (or `czor 2026.03.14-3-g1a2b3c4-dirty` during development).

Option B — **`vergen`** crate if you want richer metadata (commit hash, build timestamp). Only add this if you actually need it.

**Rules:**
- Never manually edit the version in `Cargo.toml` — leave it at `0.0.0`.
- Tags are the only version source. No tag = dev build.
- CI should fail to publish if there's no tag on the commit.

---

## Phases

Each phase ends with a **usable binary**. Don't move to the next phase until the current one works reliably.

---

### Phase 1 — Simple Splits with Pane Tracking

**Goal:** `czor split v 2:1` works correctly.

**What you build:**

1. `clap` CLI with a single `split` subcommand accepting direction + ratio string
2. Ratio parser: `"2:1"` → `vec![2, 1]`, `"1:2:1"` → `vec![1, 2, 1]`
3. A trivial `Layout` tree (one `Split` node with `Pane` leaves)
4. `weights_to_split_percentages()` — the conversion function from data structures section
5. An executor that tracks **pane IDs** from the start

**Why pane IDs matter now, not later:**
tmux assigns pane IDs (`%0`, `%1`, …) dynamically. If you target panes by position (`-t :.0`), you'll hit ordering bugs the moment layouts get complex. Instead:

```rust
fn execute(node: &Layout, target_pane: &str) -> anyhow::Result<()> {
    match node {
        Layout::Pane => Ok(()),
        Layout::Split { direction, children } => {
            // First child inherits `target_pane`
            // For each subsequent child:
            //   1. split-window on target_pane → capture new pane ID
            //   2. recurse into that child with the new ID
        }
    }
}
```

Capture the new pane ID by parsing output from:
```bash
# For a 2:1 split, the second pane gets 33% (1 out of 3 parts)
tmux split-window -v -p 33 -t %0 -P -F '#{pane_id}'
```

**Validate before moving on:**
- `czor split v 2:1` → 2 panes, top is ~67%
- `czor split h 1:1:2` → 3 panes, rightmost is ~50%
- `czor split v 1:1:1:1` → 4 equal panes
- Run `tmux list-panes -F '#{pane_id} #{pane_width} #{pane_height}'` and confirm ratios are roughly correct
- Test `weights_to_split_percentages` with known inputs:
  - `[2, 1]` → `[33]`
  - `[1, 1]` → `[50]`
  - `[1, 1, 1]` → `[50, 50]` (not `[33, 33]` — each split is relative to remainder)

**Error handling (minimum):**
- Check `$TMUX` env var is set → clear error message if not
- Check `tmux` binary exists on `$PATH`

---

### Phase 2 — Layout DSL

**Goal:** `czor layout "v(2,h(1,1))"` works.

**What you build:**

1. A recursive descent parser for the DSL
2. The `layout` subcommand

**Parser approach — hand-roll it:**

```rust
pub fn parse(input: &str) -> anyhow::Result<Layout>
```

Tokenize into: `V`, `H`, `LParen`, `RParen`, `Comma`, `Number(u32)`. Then recursive descent:

```
layout   := 'v' '(' entries ')' | 'h' '(' entries ')' | number
entries  := layout (',' layout)*
number   := bare number → Pane with associated weight
```

A bare number like `2` means "a Pane that should get 2 parts of the parent". A nested expression like `h(1,1)` consumes 1 part of the parent (its weight defaults to the implicit weight assigned by its position in the parent's child list).

**DSL weight rules:**
- Each entry in a split is `weight:child` or just `child` (default weight = 1).
- `v(2,1)` → vertical, first pane gets 2/3, second gets 1/3.
- `v(3,h(1,1))` → top pane gets 3/4, bottom half is split equally.
- `v(1,1,1)` → three equal panes (same as `v(1:pane,1:pane,1:pane)`).
- Weights are always relative to siblings. `v(2,1)` and `v(4,2)` produce identical layouts.

**Validate:**
- `v(2,h(1,1))` → 3 panes in an L-shape, top is ~67%
- `h(1,1,1)` → 3 equal columns
- `v(1,v(1,1))` → 3 rows, bottom two are half-height each
- Malformed input like `v(2,h(1,)` → clear parse error with position

---

### Phase 3 — Grid + File Input

**Goal:** `czor grid 2x3` and `czor apply layout.yaml` both work.

**Grid** is syntactic sugar — it produces a `Layout` tree, then hands it to the same executor:

```rust
fn grid(rows: u8, cols: u8) -> Layout {
    Layout::Split {
        direction: Direction::Vertical,
        children: (0..rows).map(|_| {
            (1, Layout::Split {  // equal weight per row
                direction: Direction::Horizontal,
                children: (0..cols).map(|_| (1, Layout::Pane)).collect(), // equal weight per col
            })
        }).collect(),
    }
}
```

All weights are `1` — equal splits. The grid doesn't need ratio math at all.

**File input:** Add `serde` + `serde_yaml` (or `serde_json`). Deserialize directly into `Layout`. This is straightforward if your `Layout` enum derives `Deserialize`.

```yaml
# layout.yaml — "dev" layout: big editor on top, logs + shell on bottom
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

This is equivalent to `v(2,h(1,1))` — editor gets 2/3 of height, bottom row splits equally.

**Validate:**
- `grid 2x2` → 4 equal panes
- `grid 1x4` → 4 columns
- `apply layout.yaml` matches equivalent DSL string

---

### Phase 4 — Polish

Add only what you actually want. These are independent of each other — pick and choose.

**Preview (ASCII art):**
Render the layout tree to the terminal without touching tmux. Useful for debugging.
```
czor preview "v(2,h(1,1))"
```

**Pane commands:**
Run a command in each pane after creation.
```
czor layout "v(1,1)" --cmd "nvim" --cmd "cargo watch"
```
Implementation: after the executor finishes, iterate pane IDs and `tmux send-keys`.

**Named templates:**
Store layouts in `~/.config/czor/templates/` and recall by name.
```
czor apply dev
```

**Session creation:**
Optionally create a new tmux session instead of splitting the current window.
```
czor layout "v(1,1)" --new-session my-project
```

---

## Testing Strategy

**Unit tests (from Phase 1):**
- `weights_to_split_percentages`: this is the most important unit test in the project
  - `[1, 1]` → `[50]`
  - `[2, 1]` → `[33]`
  - `[1, 2, 1]` → `[75, 50]` (second split takes 3/4 of remainder, then third takes 1/2)
  - `[1, 1, 1, 1]` → `[75, 67, 50]`
  - `[1]` → `[]` (no splits needed)
- Parser: string → `Layout` tree assertions
- Grid builder: dimensions → tree structure

**Integration tests (from Phase 2):**
```bash
tmux new-session -d -s test_session
cargo run -- layout "v(2,h(1,1))"
PANE_COUNT=$(tmux list-panes -t test_session | wc -l)
test "$PANE_COUNT" -eq 3
tmux kill-session -t test_session
```

Wrap this in a shell script or a `#[test]` that spawns/kills tmux sessions.

**Don't bother testing:**
- The tmux binary itself
- Exact pixel-level pane sizes (tmux rounds them)

---

## Common Pitfalls to Avoid

1. **Splitting the wrong pane.** Always pass an explicit `-t %ID` to every tmux command. Never rely on "the currently selected pane."
2. **Ratio math.** tmux's `-p` flag is a percentage *of the pane being split*, not of the window. For weights `[2, 1, 1]`, you don't pass `-p 50 -p 25 -p 25`. The first split takes 50% of the whole, the second takes 50% of the *remainder*. This is why `weights_to_split_percentages` exists — get it right once, test it, and never think about it again.
3. **Over-engineering the parser.** The DSL grammar fits in ~60 lines of hand-written Rust. Don't reach for `nom` until you're adding features like quoted strings or pane commands inside the DSL.
4. **Too many modules too early.** 3 files is enough through Phase 2. Split when you feel friction, not preemptively.
