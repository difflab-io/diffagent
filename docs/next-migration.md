# SDK cutover status

The legacy `diffagent-core`, Rig, Ax, and ADK implementations have been removed at the user's request. The former replacement crates now own the canonical `diffagent-rig`, `diffagent-ax`, and `diffagent-adk` package names and paths. This is a **code and entry-point cutover**, not a claim of benchmark or SDK usage parity.

## Ownership

- `diffagent-core::spec` loads the versioned YAML agent and flow configuration, prompts, skills, and parameters.
- `diffagent-core::tools` enforces workspace-scoped file policy, atomic write previews, and named sandboxed tasks. It is not complete isolation.
- `diffagent-core::events` defines model, tool, write, and flow events. `diffagent-tui` renders them together in one scrollable chat transcript, with colored Markdown and code; headings stay the same size as body text.
- Each backend owns its SDK chat loop and flow executor. Chat invokes a flow only when explicitly requested.

The provisional write event is not a disk write. A successful atomic commit emits separate confirmation. SDKs differ in how and when text fragments and usage are available; do not represent tool events as model tokens.

## Commands

```sh
mise run rig:chat                  # or ax:chat, adk:chat
mise run rig:eval:tic-tac-toe     # or any backend / fixture combination
mise run check
mise run all:eval                 # 12 paid runs; confirmation required
```

`bash scripts/chat.sh rig` uses the launch directory unless `DIFFAGENT_WORKSPACE` names another existing directory. A direct flow command is:

```sh
cargo run -p diffagent-rig -- flow implement \
  --agent agent.yaml --workspace /absolute/workspace \
  --arg 'request=Describe the work'
```

`agent-benchmark.yaml` restricts model file tools from acceptance tests, while the named test task can run them. `scripts/benchmark-flow.sh 2` saves non-overwriting results under `benchmarks/results-next/<run-set>/`. Set `DIFFAGENT_RUN_SET` to a unique name and `DIFFAGENT_FLOW_TIMEOUT_SECS` to change the default 240-second limit. Historical `benchmarks/results/` and published reports are retained. The legacy `scripts/benchmark.sh` and `diffagent.yaml` are historical and do not target the new CLI.

## Unmet evidence gate

Run a complete 12-run flow benchmark on the fixed tic-tac-toe and CSV-parser fixtures, compare external acceptance against the old observed pass count, and verify trace, prompt, source, test, and SDK usage/cost completeness before claiming parity. Timeouts count as failures. Do not infer missing SDK usage or conflate tuned pilots with independent comparisons.

Previous workspace formatting, tests, Clippy, and security tests passed before cutover. PTY smoke tests of all three chats wrote a single-player random-AI Lua program after confirmation, but Lua was not installed, so those programs were not executed. An initial six-run flow pilot passed six external acceptance checks, but two Ax flow commands failed. Later AxGen runs passed separately after prompt changes; one earlier CSV attempt timed out. These are not a successful 12-run comparison. SDK-level usage and cost artifacts are not yet at historical benchmark parity. The cutover was requested despite these open gaps.
