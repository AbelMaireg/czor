# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

- Build: `cargo build` (release: `cargo build --release`)
- Run: `cargo run -- <subcommand> <args>` (must be inside a tmux session)
- Test: `cargo test`
- Run a single test: `cargo test <test_name>` (e.g. `cargo test test_parse_nested`)
- Run tests in one module: `cargo test layout::parser::tests`
- Lint: `cargo clippy` (CONTRIBUTING expects no warnings)
- Format: `cargo fmt`

The binary refuses to run outside tmux (`TMUX` env var must be set) and shells out to the `tmux` binary, so manual end-to-end testing requires a live tmux session.

## Architecture

czor turns a declarative pane layout into a sequence of `tmux split-window` calls. The pipeline is:

```
input (CLI args / DSL string / YAML|JSON file)
  → layout::Layout tree
  → exec::execute (drives tmux)
```

### `Layout` tree (`src/layout/types.rs`)

The single intermediate representation is `Layout`, an enum of `Pane` (leaf) or `Split { direction, children: Vec<(weight, Layout)> }`. Every input source — the `split` ratio CLI form, the `layout` DSL, the `grid` shorthand, and `apply` from a file — lowers to this tree, and only `exec.rs` reads it. When adding a new input format, build a `Layout` and hand it to `exec::execute`; do not bypass the tree.

### Weight → percentage conversion (the load-bearing piece)

tmux's `split-window -p N` means "the new pane gets N% **of the pane being split**", not of the window. `weights_to_split_percentages` in `types.rs` translates sibling weights into the cascading sequence of percentages this requires. For weights `[1,1,1,1]` it produces `[75, 67, 50]`, not `[25, 25, 25]`. Tests in `types.rs` lock this behavior in — touching this function without updating those tests will silently break layouts.

### Three input frontends

- `layout::parser` — recursive-descent parser over `layout::lexer` for the DSL `v(2,h(1,1))`. Grammar lives in the doc comment on `parse`.
- `layout::file` — serde-driven YAML/JSON loader. Defines a separate `FileLayout`/`FileDirection` pair (untagged enum, lowercase direction) that converts into the core `Layout`/`Direction`. Keep the file schema decoupled from the internal types.
- `layout::types::grid` — programmatic constructor for `RxC` grids; special-cases 1×1, 1×N, and N×1 to avoid degenerate single-child splits.

### Executor (`src/exec.rs`)

`execute_node` is the only place that calls tmux. It captures the starting pane via `display-message -p '#{pane_id}'`, then for each `Split`:

1. Computes percentages from sibling weights.
2. Issues `tmux split-window <-h|-v> -p <pct> -t <target> -P -F '#{pane_id}'` once per additional child, capturing the new pane id from stdout.
3. Recurses into each child against its assigned pane id.

A single-child `Split` is collapsed (recurse without splitting) so trivial nesting in the DSL doesn't produce empty tmux operations.

## Conventions worth knowing

- `Cargo.toml` pins `edition = "2024"`.
- `serde_yaml` is the legacy 0.9 crate; expect deprecation warnings on build.
- Public surface of the `layout` module is re-exported in `src/layout/mod.rs` — add new public items there rather than referencing submodules from `main.rs`/`exec.rs`.
- The `from_file`/`from_yaml`/`from_json` re-exports are marked `#[allow(unused_imports)]` because they exist for the library API even when only `from_file` is wired into the CLI; keep that allow if you trim imports.
