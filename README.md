# diffagent: Rig vs Ax

Two separate headless Rust CLIs run the same declarative graph with the same runtime input. `crates/diffagent-rig` uses Rig; `crates/diffagent-ax` uses Ax. `crates/diffagent-core` is the shared YAML graph runner and artifact writer, not an agent provider.

## Reproduce

Set `DEEPSEEK_API_KEY` in your environment. Neither the graph nor the saved artifacts contain the key.

```sh
cargo run -p diffagent-rig -- --prompt-file examples/tic-tac-toe.md
cargo run -p diffagent-ax -- --prompt-file examples/tic-tac-toe.md
cargo test --workspace
```

Use `--prompt 'your own request'` or `--prompt-file -` to supply input at runtime. Neither CLI assumes tic-tac-toe. Both find `diffagent.yaml` in the current directory or an ancestor, then fall back to `~/.difflab/diffagent.yaml`.

## What to inspect

- [`diffagent.yaml`](diffagent.yaml) defines **plan → implement → evaluate**, with **FAIL → update → evaluate** (at most one update) and **PASS → end**.
- [`prompts/`](prompts/) contains the prompt templates. `{{input}}` is the incoming runtime request; later stages also receive earlier outputs through `{{plan}}`, `{{implementation}}`, and `{{evaluation}}`.
- [`runs/rig/`](runs/rig/) and [`runs/ax/`](runs/ax/) contain actual runs using the same tic-tac-toe input. Each includes `input.md`, the rendered `*.prompt.md`, generated `plan.md`, `implement.md` and extracted `implement.rs`, model-review `evaluate.md`, and a `trace.json` with node order and per-call elapsed time. A failed evaluation also produces `update.md` and `evaluate-after-update.md`. Rerunning a CLI replaces that backend's artifact directory.
- [`COMPARISON.md`](COMPARISON.md) compares these particular runs. Timings are observations, **not benchmarks**.

**Important limitation:** These are text-generation backends, not autonomous coding agents. Neither backend has file-write, shell, or test tools. The model-generated Rust implementation is saved as Markdown and extracted to `.rs`, but is not compiled or run. `PASS` means an LLM reviewer approved it; it does not establish correctness. We have not implemented Claude Code/Cursor CLI orchestration, Pi RPC, or a tool-execution security policy in this POC.
