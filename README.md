---
base_commit: 'e806da18c22ef10caa8d9a846b760d7fe6ce2cf5'
---

# PoC: Compare Rig, Ax, and ADK-Rust agents

> Experimental branch only. Do not merge into production or publish as a release.

## Purpose

Can three Rust agent SDKs support separate chat loops and the same parameterized YAML coding flow with comparable tool behavior and external test results?

The smallest useful result is a runnable chat and flow for each SDK, followed by the same fixed acceptance tests and complete usage records. This PoC does not provide safe execution for hostile users or a multiuser remote-task service. The final 12-run comparison and usage/cost parity are still pending.

## Setup and tasks

Run commands from the repository root on macOS. Install [mise](https://mise.jdx.dev/) first; `mise install` installs the pinned Rust 1.98.1 (including Cargo), Python 3.12.13, and Go 1.25.14 toolchains declared in [`mise.toml`](mise.toml). Rust dependencies are locked in `Cargo.lock` and download on the first build. The Python benchmark helpers use only the standard library, and the bundled Go example uses Go's standard library. Bash and `/usr/bin/sandbox-exec` come with macOS, not mise; the sandbox is defense in depth, not isolation for hostile code. No local database, Node.js, or Lua installation is required to run the PoC.

```sh
mise install
mise run build
mise run test
# For chat and paid evaluations, first set DEEPSEEK_API_KEY in your shell.
mise run preflight                   # checks tools, sandbox, and key presence; no API call
mise run rig:chat                    # or ax:chat, adk:chat
mise run rig:eval:tic-tac-toe       # paid; asks for confirmation
```

A DeepSeek account with API access and network connectivity is required for chat and evaluations. Set `DEEPSEEK_API_KEY` in your environment, not in a file in this repository; do not commit it. The key is not forwarded to agent-run mise tasks. `mise run preflight` checks only that the key is present, not that it works. Build, test, check, and preflight do not call the model provider. Each evaluation task asks for confirmation and writes a new set under `benchmarks/results-next/`, which Git ignores without deleting the results. Historical `benchmarks/results/` files also remain on disk; Git ignore rules do not untrack files already committed.

| Environment variable | When needed | Effect |
| --- | --- | --- |
| `DEEPSEEK_API_KEY` | Required for chat and evaluations. | DeepSeek model calls; never store it in the repository. |
| `DIFFAGENT_WORKSPACE` | Optional for `scripts/chat.sh` and chat mise tasks. | Absolute path to an existing workspace; defaults to the directory where the chat command starts. |
| `DIFFAGENT_RUN_SET` | Optional for `scripts/benchmark-flow.sh`; mise evaluation tasks set it automatically. | Unique result-set name containing only letters, digits, `_`, or `-`; the script otherwise uses `current` and refuses to overwrite a run. |
| `DIFFAGENT_FLOW_TIMEOUT_SECS` | Optional for flow benchmarks. | Positive integer timeout per flow, default `240` seconds. |

| Task | Command | Purpose | Observed result |
| --- | --- | --- | --- |
| Build | `mise run build` | Compile the workspace. | Passed with pinned tools. |
| Test | `mise run test` | Run Rust workspace and Go example tests without model calls. | Passed with pinned tools. |
| Check | `mise run check` | Check Rust formatting, Rust and Go tests, and Clippy. | Passed with pinned tools. |
| Preflight | `mise run preflight` | Check runtime tools, macOS sandbox, and key presence without model calls. | Passed with a placeholder key; live key validity remains unverified. |
| Run Rig | `mise run rig:chat` | Open Rig chat. | CLI startup passed in an earlier PTY smoke test; no live chat completion verified here. |
| Run Ax | `mise run ax:chat` | Open Ax chat. | CLI startup passed in an earlier PTY smoke test; no live chat completion verified here. |
| Run ADK | `mise run adk:chat` | Open ADK chat. | CLI startup passed in an earlier PTY smoke test; no live chat completion verified here. |
| Pilot | `mise run rig:eval:tic-tac-toe` | Run one paid Rig acceptance case; replace the backend or fixture to run the other five combinations. | Not run in this check. |
| Full comparison | `mise run all:eval` | Run 12 paid flow evaluations after confirmation. | Pending authorization and verification. |

The chat script uses the directory where you launch it as the workspace. When you run a mise task here, the workspace is this repository. Set `DIFFAGENT_WORKSPACE=/absolute/existing/path` to select another workspace. The sample `agent.yaml` permits reads and writes across that workspace, including hidden files and Go files. Do not choose a workspace with secrets that the agent must not read. Tools reject paths outside the workspace, symlinks, reads over 32 KiB, and writes over 256 KiB.

The terminal puts messages, tool calls, output, and write diffs in one transcript. Markdown headings use color without larger text. Code and diffs show at most 12 lines inline; click an entry or press Ctrl+O for the full content. Press Ctrl+Y to copy the full transcript or the full open entry. Press Esc to close the view, Up/Down to scroll, PageUp/PageDown to jump ten lines, and Shift-Enter for a newline. Press Ctrl+T to toggle reasoning text only when the SDK exposes it. Rig and ADK can expose reasoning during chat; Ax's high-level callback currently exposes answer text but not reasoning.

Chat starts a named YAML flow only when you request it. Agents can ask a question in chat; select an option, type an answer, or press Esc to skip. A flow asks only when input and error output connect to a terminal. Headless flows skip questions, and `questions: skip` disables them in the YAML file. Write previews are provisional; a separate event confirms an atomic commit.

Direct commands have the same shape for each backend:

```sh
cargo run -p diffagent-rig -- chat --agent agent.yaml --workspace /absolute/workspace
cargo run -p diffagent-rig -- flow implement --agent agent.yaml --workspace /absolute/workspace --arg 'request=Describe the work'
```

Replace `rig` with `ax` or `adk`. A task in the workspace `mise.toml` can run shell commands, but the agent has no direct shell tool. The host runs discovered mise tasks without the model key, with a 180-second timeout and an 8 KiB output cap. The macOS sandbox blocks network access and writes outside the workspace, but it does not block every read outside the workspace.

## Code map

`crates/diffagent-core` contains `spec` for YAML loading and parameter validation, `tools` for workspace file and task policy, and `events` for progress and question handling. `crates/diffagent-tui` renders their events. Each SDK crate has `agent.rs` (`define_agent`), `tools.rs` (`define_tool` and `define_tools`), `workflows.rs` (`define_flow`), `chat.rs`, and a CLI in `main.rs`. No shared agent loop or flow executor hides SDK differences.

`agent.yaml` selects the model, prompts, skills, flows, file policy, and named tasks. `flows/implement.yaml` and its prompt files define the sample coding flow. `agent-benchmark.yaml` restricts model file tools from the external acceptance tests while its named test task can still run them. `benchmarks/fixtures/` holds the tic-tac-toe and CSV-parser inputs and acceptance tests. `scripts/benchmark-flow.sh` runs the explicit flow for the selected backend and fixture.

## Alternatives

- Rig registers typed `Tool` implementations with an `AgentBuilder` and runs flow agent nodes through Rig's async agent loop.
- Ax constructs tool handlers and selects `AxAgent` for chat or `AxGen` for flow nodes.
- ADK wraps `FunctionTool` handlers in an `LlmAgent` and runs agent nodes through an ADK `Runner`.

## Experiments

| Approach | What changed | How to run | Result |
| --- | --- | --- | --- |
| Rig | SDK-owned chat and YAML flow executor. | `mise run rig:eval:tic-tac-toe` | Offline tests pass; a fresh paid evaluation is pending. |
| Ax | SDK-owned chat and YAML flow executor. | `mise run ax:eval:tic-tac-toe` | Offline tests pass; a fresh paid evaluation is pending. |
| ADK | SDK-owned chat and YAML flow executor. | `mise run adk:eval:tic-tac-toe` | Offline tests pass; a fresh paid evaluation is pending. |

Run each approach on both fixtures for a comparison. The six individual `*:eval:*` tasks run one paid case each. `mise run all:eval` runs two repetitions of every backend and fixture combination. Set `DIFFAGENT_RUN_SET` for a unique result name or `DIFFAGENT_FLOW_TIMEOUT_SECS` to change the default 240-second flow limit. Do not overwrite `benchmarks/results/` or the published historical reports.

## Learnings

The [historical full-agent summary](benchmarks/AGENTS.md), [machine-readable data](benchmarks/agents.json), [earlier Rig-versus-AxGen comparison](benchmarks/SUMMARY.md), and saved artifacts describe legacy implementations, not the current SDK crates. A later six-case replacement pilot passed external acceptance checks, but two Ax flow commands failed. These pilots do not establish 12-run parity or comparable SDK usage and cost reporting. See [the cutover status](docs/next-migration.md) for the remaining evidence gaps. The current workspace build, tests, and Clippy pass, but a live Rig question path timed out before its answer was verified end to end. Lua output was inspected but not executed because Lua was unavailable locally.

## Compare with main at branch creation

This branch existed before `origin/main`. Its `base_commit` above is the earliest PoC commit, chosen as a retrospective, ancestor baseline. It is **not** a claim that `origin/main` existed at branch creation. The later `origin/main` root commit is unrelated to this branch, so the standard Difflab PoC ancestry check cannot pass without a separate history decision. Compare committed work since the recorded baseline with `git diff e806da18c22ef10caa8d9a846b760d7fe6ce2cf5 HEAD`; add a working-tree diff to inspect uncommitted changes.

## Disposition

Keep this as an experimental branch. Do not merge it through the PoC workflow. Run and assess the authorized 12-case evaluation and the usage/cost gate before choosing a backend or freezing a tag. Explore multiuser remote execution on a separate PoC branch.
