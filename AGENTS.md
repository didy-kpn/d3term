# Repository Guidelines

## Project Structure & Module Organization

This repository is a Tauri terminal app (`d3term`) with a TypeScript frontend and Rust backend.

- `src/`: frontend app code (`main.ts`, `terminal.ts`, `config-client.ts`, `styles.css`)
- `src/*.test.ts`: frontend unit tests (Vitest)
- `src-tauri/src/`: backend runtime (`main.rs`, `commands.rs`, `pty.rs`, `config.rs`, `state.rs`)
- `config/config.example.toml`: user configuration template
- `docs/`: product docs (`requirements.md`, `design.md`)
- `plans/`: implementation planning notes

Keep feature changes split clearly between `src/` (UI/rendering) and `src-tauri/` (PTY/config/runtime).

## Build, Test, and Development Commands

- `npm install`: install JS dependencies
- `npm run tauri dev`: run the full desktop app (recommended for real behavior)
- `npm run dev`: browser-only preview (no PTY backend)
- `npm run build`: TypeScript check + production frontend build
- `npm run test`: run frontend tests with Vitest
- `cd src-tauri && cargo test`: run Rust unit tests
- `cd src-tauri && cargo fmt --check`: verify Rust formatting

## Coding Style & Naming Conventions

- TypeScript: 2-space indentation, strict typing, ES modules
- Rust: rustfmt defaults (4-space indentation), idiomatic module split
- File naming: kebab-case in frontend (for example `config-client.ts`), snake_case in Rust modules
- Naming: `PascalCase` for types/interfaces, `camelCase` for functions/variables
- Keep comments short and only for non-obvious behavior

## Testing Guidelines

- Frontend tests use Vitest and should live near source as `*.test.ts`
- Backend tests live in `#[cfg(test)]` modules in each Rust file
- Add regression tests for config parsing, command selection, and fallback behavior
- Before opening a PR, run both:
  - `npm run test`
  - `cd src-tauri && cargo test`

There is no enforced coverage threshold yet; prioritize meaningful behavior tests.

## Commit & Pull Request Guidelines

Git history is minimal (`Initial commit`), so no strict commit format is established yet. Use clear, imperative commit subjects with a scope, for example:

- `frontend: tune dark theme palette`
- `backend: improve zellij fallback warning`

For pull requests, include:

- What changed and why
- Key files touched
- Test commands and results
- Screenshot(s) for UI/theme/typography changes
- Any config impact (for example `config.toml` fields)

## Security & Configuration Tips

- Do not commit secrets, tokens, or local machine paths
- Treat `config/config.example.toml` as the source of truth for documented options
- Keep safe fallback behavior when changing startup command logic (`tmux`/`zellij` -> shell fallback)
