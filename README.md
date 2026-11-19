# Diffagent

Diffagent is an open-source desktop and CLI (command-line interface) workspace for declarative local and cloud agents that synchronize with Git.

## Repository layout

```text
diffagent/
├── apps/
│   ├── diffagent-cli/       Rust Clap CLI (`diffagent`)
│   ├── diffagent-desktop/   Tauri v2 desktop host and Specta binding exporter
│   └── diffagent-web/       Static SvelteKit 5 shell (`WhichKey`, `CommandPalette`, `CommandMenu`)
├── libs/
│   └── diffagent-ui/        Svelte 5 component library built on `@cloudvoyant/helical-ui`
├── docs/
│   ├── architecture/        Software architecture documentation
│   └── environment.md       Environment variable reference
└── e2e/
    ├── common/              Isolated `DIFFAGENT_HOME` harness utilities
    ├── cli/                 Vitest CLI end-to-end suite (`cli.log`)
    └── desktop/             Playwright Chromium and WebKit desktop suite (`ipc.log`)
```

## Quick start

Install [Mise](https://mise.jdx.dev) and [Premise](https://github.com/cloudvoyant/premise), then run workspace tasks from the repository root:

```sh
premise run install
premise run build
premise run lint
premise run test
premise run e2e:smoke
premise run e2e
```

Launch the CLI or desktop shell locally during development:

```sh
mise run run:cli -- status --json
mise run dev:web
mise run dev:desktop
```

## Documentation

- [Software architecture](docs/architecture/index.md)
- [Environment variables](docs/environment.md)
- [End-to-end test suites](e2e/README.md)
- [Repository conventions](AGENTS.md)
