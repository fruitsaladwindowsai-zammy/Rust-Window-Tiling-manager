# Contributing

Contributions are welcome!

## Setup & Checks

1. Install **Rust 1.85+** and **MSVC Build Tools** on Windows.
2. Create a feature branch from `main`.
3. Run the verification suite before opening a PR:

```powershell
cargo fmt -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo check --release --workspace --tests
```

## Guidelines

- Keep OS-agnostic grid, layout, config, and persistence logic inside `crates/winscroller2d-core` with unit tests in `crates/winscroller2d-core/tests/layout_tests.rs`.
- Keep documentation in sync (`winscroller2d.example.toml`, `KEYBINDINGS.md`, `CONFIGURATION.md`, `docs/src/`).
- Contributions are licensed under the [MIT License](https://opensource.org/licenses/MIT).
