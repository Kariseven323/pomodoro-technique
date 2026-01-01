# Repository Guidelines

- must use chinese to reply
- comment must be written for every method and function

## Project Structure & Module Organization

This repo is a Windows Pomodoro desktop app built with **Tauri 2 (Rust)** + **SvelteKit 5** + **Vite** + **Tailwind CSS 4**.

- `src/`: SvelteKit frontend
  - `src/routes/+page.svelte`: main UI (timer, tags, stats)
  - `src/lib/components/`: modal and UI components
  - `src/lib/tauriApi.ts`: wrapper for Tauri `invoke()` calls
  - `src/lib/types.ts`: shared types
- `src-tauri/`: Rust backend (Tauri core)
  - `src-tauri/src/lib.rs`: app setup, plugins, command bindings
  - `src-tauri/src/commands.rs`: Tauri commands exposed to the frontend
  - `src-tauri/src/timer.rs`: timer state machine (work/short/long)
  - `src-tauri/src/processes.rs`: process enumeration/termination (Windows APIs)
  - `src-tauri/tauri.conf.json`: Tauri configuration
- `docs/`: PRDs and bug notes (e.g. `docs/prd/`)

## Build, Test, and Development Commands

- `bun install`: install frontend dependencies
- `bun run tauri dev`: run frontend + Tauri in dev mode (hot reload)
- `bun run dev`: run only the Vite dev server
- `bun run check`: sync SvelteKit and run `svelte-check` (type/diagnostics)
- `bun run tauri build`: produce a production desktop build
- `cd src-tauri && cargo fmt`: format Rust code
- `cd src-tauri && cargo clippy`: lint Rust code
- `cd src-tauri && cargo test`: run Rust unit tests (when present)

## Coding Style & Naming Conventions

- TypeScript/Svelte: use 2-space indentation; prefer `camelCase` for variables/functions and `PascalCase` for components.
- Rust: `cargo fmt` defaults; prefer `snake_case` and keep Windows-only code behind `cfg(windows)` when applicable.
- Keep filenames descriptive (e.g. `timer.rs`, `tauriApi.ts`), and avoid introducing new patterns without updating docs.

## Testing Guidelines

There is no dedicated JS test runner configured yet. For now:

- Run `bun run check` before PRs.
- Add Rust unit tests near the module (`src-tauri/src/*.rs`) and run `cargo test`.
- For UI changes, include a short manual QA note (steps + expected behavior).

## Commit & Pull Request Guidelines

- Commits follow a Conventional-Commits style seen in history: `feat: ...`, `fix: ...`, `docs: ...` (Chinese summaries are OK).
- PRs should include: purpose, linked issue (if any), Windows 10/11 verification notes, and screenshots/GIFs for UI changes.

## Agent-Specific Instructions

- When using coding assistants/agents, prefer replies in Chinese.
- Add a short Chinese doc comment for every new/changed function or method (`///` in Rust, `/** ... */` in TS).
