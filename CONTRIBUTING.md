# Contributing to czor

Thank you for your interest in contributing to czor! This document provides guidelines and information for contributors.

## How to Contribute

### Reporting Bugs

If you find a bug, please open an issue on GitHub with:

- A clear, descriptive title
- Steps to reproduce the issue
- Expected vs actual behavior
- Your tmux version (`tmux -V`)
- Your Rust version (`rustc --version`)

### Suggesting Features

Feature suggestions are welcome! Please open an issue with:

- A clear description of the feature
- Use cases and examples
- Any implementation ideas you might have

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Run clippy (`cargo clippy`)
6. Format code (`cargo fmt`)
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to your branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

## Development Setup

```bash
# Clone your fork
git clone https://github.com/AbelMaireg/czor.git
cd czor

# Build the project
cargo build

# Run tests
cargo test

# Run with debug output
cargo run
```

## Code Style

- Follow standard Rust conventions
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes without warnings
- Add tests for new functionality

## Testing

Tests require tmux to be installed. Run tests with:

```bash
cargo test
```

## License

By contributing to czor, you agree that your contributions will be licensed under the MIT License.
