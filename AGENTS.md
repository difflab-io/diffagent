# Repository Guidance

## Hard Rules

- Run builds, formatters, linters, and tests through root [Mise tasks](mise.toml) via Premise (`premise run format`, `premise run lint`, `premise run test`, `premise run build`, `premise run e2e:smoke`, `premise run e2e`).
- Scaffold workspace projects and workflows with [Premise](premise.yaml) and author design, architecture, plan, and review documents from Difflab templates.
- Keep `apps/diffagent-desktop` excluded from the root [Cargo workspace](Cargo.toml) so Tauri native dependencies stay isolated from CLI builds.
- Regenerate `apps/diffagent-web/src/lib/gen/bindings.ts` with `cargo run --manifest-path apps/diffagent-desktop/Cargo.toml -- --export-bindings` whenever Tauri commands or Specta types change. Never edit generated bindings by hand.
- Isolate every worktree task and automated test from `~/.diffagent` via `DIFFAGENT_HOME`. Keep `assertIsolatedTestHome` in place before any test directory cleanup.
- Keep `diffagent open` argumentless. Opening the desktop application takes no path or workspace arguments.

## Organization

### Repository layout

- Place deployable binaries and shells under `apps/` (`diffagent-cli`, `diffagent-desktop`, `diffagent-web`), shared packages under `libs/` (`diffagent-ui`), TypeScript end-to-end suites under `e2e/` (`common`, `cli`, `desktop`), and maintained references under [`docs/`](docs/architecture/index.md).
- Retain git-ignored E2E artifacts under `e2e/outputs/<suite>/<spec>/<test-slug>/` (`cli.log` or `ipc.log` plus `diffagent-home/`) after both passing and failing runs for direct inspection.

### File layout

- Keep one responsibility per module and service. Wire cross-service events at application entry points (`apps/diffagent-desktop/src/app.rs` and `apps/diffagent-web/src/lib/components/Shell.svelte`).
- In Rust types, use one inherent `impl` block per type with constructors first, `// Public` methods second, `// Private` helpers last, and trait `impl` blocks afterward. Pad section comments with hyphens to column 80.
- [Proposed] Place public functions, methods, and getters ahead of private helpers in Rust modules and TypeScript service classes.
- In Svelte 5 components, group `<script lang="ts">` sections with padded `// Props`, `// State`, `// Lifecycle`, and `// Methods` headers.

## Style

### Rust

- Enable `#![warn(missing_docs, unreachable_pub)]` in crate roots and document crate-visible and public items with `///` docstrings stating boundary purpose.
- Use `pub(crate)` for internal modules and items inside binary crates (`diffagent-cli`) so `-D warnings` stays clean.
- Place CLI output structs under `apps/diffagent-cli/src/models/`, name command payload wrappers `*CommandOutput`, and render both human and `--json` output through `print` methods.

### TypeScript and Svelte

- Document exported TypeScript classes, interfaces, and functions with `/** ... */` JSDoc comments.
- Compose `@cloudvoyant/helical-svelte` and `@cloudvoyant/helical-ui` primitives directly in `apps/diffagent-web` (`WhichKeyProvider`, `WhichKey`, `WhichKeyMode`, `CommandProvider`, `CommandPalette`, `CommandMenu`, `SidebarProvider`, `Pane`).
- Build app-agnostic Svelte 5 primitives in `libs/diffagent-ui` with `@ark-ui/svelte`, `class-variance-authority` (`styles.ts`), semantic Tailwind tokens, and standalone `types.ts` contracts suitable for upstreaming to `helical-ui`.
- Keep `apps/diffagent-web/src/app.css` restricted to Tailwind v4 and stock `@cloudvoyant/helical-ui` stylesheet imports plus base resets.

### Documentation

- Document all runtime and test environment variables in [`docs/environment.md`](docs/environment.md) and system boundaries in [`docs/architecture/index.md`](docs/architecture/index.md).
- Write documentation in plain, active-voice English with soft-wrapped paragraphs, no bold list lead-ins, and Mermaid diagrams or tables over long prose.
- Preserve Difflab template headings in `docs/architecture/index.md` without `FR` or `NFR` bullet prefixes under `## Requirement`.
- State `## Requirement` bullets in architecture docs as client-facing behavioral and performance characteristics rather than internal implementation specs.
- Omit UI keyboard shortcut and keybinding reference tables from architecture docs.

## Testing

- Write all end-to-end tests in TypeScript under [`e2e/`](e2e/README.md) (`e2e/cli` with Vitest and `e2e/desktop` with Playwright across Chromium and WebKit). Never add Rust E2E suites under `apps/*/tests/`.
- Keep UI primitive unit tests upstream in `helical-ui`. Do not add CVA class-string assertions, static markup tests, or test-harness self-tests in this repository; `libs/diffagent-ui` and `apps/diffagent-web` run `bun test --pass-with-no-tests`.
- Structure stateless command specs action-first (`describe('when <action>', ...)` outside with flat `it('should <outcome> given <condition>')` leaves) and group stateful desktop shell specs under `test.describe('given a mounted desktop shell over --ipc-bridge', ...)` with flat `<subject> should <outcome>` leaves.
- Separate `// Arrange`, `// Act`, and `// Assert` blocks inside every test body. Run `premise run e2e:smoke` for fast feedback during development and `premise run e2e` before completion.
