# Scaffold — next-prisma

This adapter uses `create-next-app` combined with `prisma init` to generate the initial project scaffold at pipeline runtime. The `--scaffold-source` CLI flag can override the scaffold source path.

## Usage

The `factory-run` CLI handles scaffold creation during the `s6a-scaffold-init` stage:
1. Runs `npx create-next-app@latest` with TypeScript and App Router options
2. Runs `npx prisma init` to set up the Prisma schema and migrations directory
