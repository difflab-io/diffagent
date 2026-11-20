# Diffagent environment variables

## Overview

Diffagent applications (`diffagent-cli` and `diffagent-desktop`) and the TypeScript `e2e/` test suites read environment variables to isolate state directories, locate binaries, control logging, and select test subsets.

## Runtime configuration

| Variable                | Component                            | Default                                                                                                                          | Description                                                                                                                                        |
| ----------------------- | ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `DIFFAGENT_HOME`        | `diffagent-cli`, `diffagent-desktop` | `~/.diffagent` (`<worktree>/.tmp/diffagent-home` inside `mise` tasks)                                                            | Overrides the Diffagent home directory reported by `diffagent status` and `app_info` and forwarded to child desktop processes by `diffagent open`. |
| `DIFFAGENT_DESKTOP_BIN` | `diffagent-cli`                      | Auto-discovered (sibling binary, `apps/diffagent-desktop/target/{debug,release}`, `PATH`, `/Applications/diffagent-desktop.app`) | Overrides the `diffagent-desktop` executable path launched by `diffagent open`.                                                                    |
| `DIFFAGENT_WEB_PORT`    | `diffagent-web`, `diffagent-desktop` | `1420` (falls back to a free `127.0.0.1` port when busy)                                                                         | Overrides the Vite development server port and Tauri `devUrl` used by `dev:web` and `dev:desktop`.                                                 |
| `RUST_LOG`              | `diffagent-cli`                      | `warn`                                                                                                                           | `tracing_subscriber::EnvFilter` directive controlling diagnostic output written to `stderr`.                                                       |

## End-to-end test harness (`e2e/`)

| Variable                 | Component                                      | Default                      | Description                                                                             |
| ------------------------ | ---------------------------------------------- | ---------------------------- | --------------------------------------------------------------------------------------- |
| `DIFFAGENT_IPC_BRIDGE`   | `e2e/desktop/fixtures.ts`, `diffagent-desktop` | Unset (`1` under Playwright) | Marks subprocess execution inside the `diffagent-desktop --ipc-bridge <home>` harness.  |
| `DIFFAGENT_E2E_EXTENDED` | `e2e/common/config.ts`                         | `false`                      | Includes `@extended` Vitest cases when set to `true`.                                   |
| `DIFFAGENT_E2E_SPEC`     | `e2e/common/config.ts`                         | `**/*.spec.ts`               | Restricts Vitest CLI discovery to a single glob or spec file.                           |
| `CARGO_TARGET_DIR`       | `e2e/common/test-harness.ts`                   | `<repo>/target`              | Overrides the Cargo build directory used to locate the compiled `diffagent` CLI binary. |

## References

- [Software architecture](architecture/index.md).
- [End-to-end test suite guide](../e2e/README.md).
- [Workspace task configuration](../mise.toml).
