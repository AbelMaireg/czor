# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-05-02

### Added

- `split` subcommand — divide the current pane with a ratio (e.g. `v 2:1`, `h 1:1:1`)
- `layout` subcommand — apply a nested layout DSL (e.g. `"v(2,h(1,1))"`)
- `grid` subcommand — create RxC grids (e.g. `2x3`)
- `apply` subcommand — load a layout from a YAML or JSON file
- Weight-to-percentage conversion that accounts for tmux's cascading split model
- MIT license, CONTRIBUTING guide, and Code of Conduct
