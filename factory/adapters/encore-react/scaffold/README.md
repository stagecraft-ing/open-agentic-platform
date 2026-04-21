# Scaffold — encore-react

This adapter uses `encore app create` to generate the initial project scaffold at pipeline runtime. The `--scaffold-source` CLI flag overrides the default scaffold behavior.

Unlike template-based adapters, Encore projects are initialized through the Encore CLI which sets up the runtime, database migrations, and service structure automatically.

## Usage

The `factory-run` CLI handles scaffold creation during the `s6a-scaffold-init` stage. No manual scaffold directory is required.
