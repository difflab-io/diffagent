# diffagent: Rig vs Ax tool-calling POC

Two separate Rust CLIs run the same `diffagent.yaml` graph and the same runtime request. `diffagent-rig` uses Rig's agent runner and typed tools; `diffagent-ax` uses AxGen's tool handlers and iterative model calls. The shared `diffagent-core` owns the graph, restricted tools, artifact files, and measurement format.

## Run

Set `DEEPSEEK_API_KEY` in your environment; it is not written to the repository. This POC **requires macOS `sandbox-exec` to run generated tests** and refuses that task without it. Generated code runs under your user account; the sandbox restricts network and file writes outside the generated workspace and blocks reads of some credential directories, but it is **not a VM or complete hostile-code isolation**. Do not run untrusted prompts on a machine with sensitive data.

```sh
cargo run -p diffagent-rig -- --prompt-file examples/tic-tac-toe.md
cargo run -p diffagent-ax -- --prompt-file examples/tic-tac-toe.md
cargo test --workspace
```

You can pass an arbitrary request via `--prompt '...'`, or pipe it through `--prompt-file -`. The CLIs search for `diffagent.yaml` in the current directory and ancestors, then `~/.difflab/diffagent.yaml`.

## Fixed-task comparison

To reproduce the two-task, two-repetition comparison with independent acceptance tests, run `scripts/benchmark.sh 2`. The fixtures in [`benchmarks/fixtures/`](benchmarks/fixtures/) specify an exact Rust API and contain tests outside the model's permitted read/write tool paths. Each run is saved under `benchmarks/results/<task>/<backend>/<run>/`, including the generated source, tool calls, trace, usage, and cost estimate. See the [eight-run summary](benchmarks/SUMMARY.md), [provisional decision](benchmarks/DECISION.md), and machine-readable [`summary.json`](benchmarks/summary.json). This still does not make generated code safe to run outside an isolated environment.

## What runs

1. **plan**: One model call produces a short plan using `{{input}}`.
2. **implement**: An iterative agent can call `read_file`, `write_file`, and `run_task`. Both adapters give these tools to the model; the host only lets it read `src/lib.rs` or `Cargo.toml`, write `src/lib.rs`, and run `task=test` (`cargo test --offline --quiet` with a 45-second limit). The model sees test failures and can edit and retry within its run.
3. **evaluate**: The host independently reruns the same test task against the actual written source. On failure, the graph routes to **update** (one revision stage), then reevaluates; on success, it ends. Evaluation is **not another model's opinion**.

Every run replaces only its own `runs/<backend>/` directory. Compare the actual [`Rig source`](runs/rig/workspace/src/lib.rs) and [`Ax source`](runs/ax/workspace/src/lib.rs), [tool-call histories](runs/rig/tools.json), per-step [`traces`](runs/rig/trace.json), and [`metrics`](runs/rig/metrics.json). The Ax run also saves normalized per-call [`usage records`](runs/ax/usage-agent.json). `*.prompt.md` shows the exact input to each model stage. [`COMPARISON.md`](COMPARISON.md) summarizes the saved runs.

Tokens count all model calls, including tool loops. Cache-hit tokens are included in **input** tokens. Cost is an **estimate**, not billing: `metrics.json` applies DeepSeek's published `deepseek-flash` off-peak and peak rates to reported input, cache-hit and output tokens. See [DeepSeek pricing](https://api-docs.deepseek.com/quick_start/pricing/). The outputs and timings vary from run to run; even the repeated fixed-task runs are a small sample, not a controlled performance benchmark. Passing tests authored by the model does not establish that all user requirements are met.

This intentionally narrow tool surface does not yet provide general shell commands, multi-file projects, arbitrary graph nodes, session persistence, or Claude Code/Cursor/Pi adapters. Test execution has OS- and threat-model limitations; do not deploy the POC as a general-purpose autonomous coder.
