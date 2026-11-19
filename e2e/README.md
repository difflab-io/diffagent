# End-to-End Specifications

TypeScript end-to-end suites verify `diffagent-cli` and `diffagent-desktop` from the outside in. CLI suites run Vitest against the compiled `diffagent` binary, and desktop suites run Playwright in Chromium and WebKit while routing every Tauri IPC call and event over `diffagent-desktop --ipc-bridge`.

## Layout

```text
e2e/
├── common/
│   ├── config.ts                    shared Vitest configuration and tag definitions
│   ├── fixtures.ts                  per-test lifecycle hook for CLI journeys
│   ├── test-harness.ts              isolated subprocess runner and cli.log recorder
│   └── test-utils.ts                assertIsolatedTestHome, isolatedEnv, and waitFor
├── cli/
│   ├── mise.toml                    CLI E2E Mise tasks
│   ├── vitest.config.ts             CLI Vitest entry configuration
│   ├── help.spec.ts                 bare diffagent, --help, --version, status, and unknown command checks
│   └── open.spec.ts                 diffagent open desktop binary resolution and detached launch
├── desktop/
│   ├── mise.toml                    Playwright install, bridge build, and browser test tasks
│   ├── playwright.config.ts         Chromium and WebKit configuration
│   ├── vite.config.ts               isolated Shell.svelte test bundle configuration
│   ├── fixture.{html,ts}            mockIPC transport wiring into Shell.svelte
│   ├── fixtures.ts                  stdio --ipc-bridge process runner and ipc.log recorder
│   └── shell.spec.ts                startup, WhichKey/hotkeys, and CommandPalette/CommandMenu checks
└── outputs/<suite>/<spec>/<slug>/   retained git-ignored artifacts per test
    ├── cli.log                      CLI command transcripts (cli suite)
    ├── ipc.log                      stdio IPC request, response, and event frames (desktop suite)
    └── diffagent-home/              per-test isolated DIFFAGENT_HOME directory
```

## Commands

| Task          | Command                   | Purpose                                                                                    |
| ------------- | ------------------------- | ------------------------------------------------------------------------------------------ |
| Smoke E2E     | `premise run e2e:smoke`   | Run `@smoke` CLI and desktop journeys.                                                     |
| Full E2E      | `premise run e2e`         | Run all CLI and desktop suites in parallel.                                                |
| CLI E2E       | `premise run e2e:cli`     | Build `diffagent` and run `e2e/cli` specs.                                                 |
| Desktop E2E   | `premise run e2e:desktop` | Build `diffagent-desktop --features e2e-bridge` and run Playwright in Chromium and WebKit. |
| List journeys | `premise run e2e:list`    | List CLI and desktop test cases without running them.                                      |

## Inspecting Retained Test Directories

Every test recreates `e2e/outputs/<suite>/<spec>/<slug>/` before running, terminates child processes on teardown, and leaves `cli.log`, `ipc.log`, and `diffagent-home/` on disk for direct inspection:

```sh
cd e2e/outputs/cli/help/status-json-should-return-version-and-home-dir-without-mutating-diffagent-home
cat cli.log
ls -la diffagent-home
```
