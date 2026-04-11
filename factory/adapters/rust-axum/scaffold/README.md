# Scaffold — rust-axum

This adapter uses `cargo init` combined with axum dependencies to generate the initial project scaffold at pipeline runtime. The `--scaffold-source` CLI flag can override the scaffold source path.

## Usage

The `factory-run` CLI handles scaffold creation during the `s6a-scaffold-init` stage:
1. Runs `cargo init` to create the Rust project structure
2. Adds axum, tokio, serde, and sqlx dependencies to `Cargo.toml`
3. Creates the directory layout per the manifest's `directory_conventions`
