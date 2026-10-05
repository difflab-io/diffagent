# Rig vs Ax: initial smoke runs

For the stronger two-task, eight-run comparison with external acceptance tests, see [benchmarks/SUMMARY.md](benchmarks/SUMMARY.md). This page preserves the earlier smoke-run observations, where each agent wrote its own tests.

Both separate CLIs received [`examples/tic-tac-toe.md`](examples/tic-tac-toe.md), followed [`diffagent.yaml`](diffagent.yaml), used `deepseek-flash`, and had the same three host tools. These are **single observed runs**, not a benchmark or statistical evaluation. Models generated different-sized implementations and different tests.

| Recorded observation | Rig CLI | Ax CLI |
|---|---:|---:|
| Model calls (plan + iterative implementation) | 9 | 5 |
| Input tokens, **including cache hits** | 126,519 | 9,647 |
| Cached input tokens | 112,512 | 8,448 |
| Output tokens | 25,232 | 3,491 |
| Host tool calls, including final evaluation | 9 | 5 |
| In-agent test attempts before PASS | 2 (1 failure) | 1 (PASS) |
| Final host `cargo test` | PASS: 24 tests | PASS: 9 tests |
| Sum of stage times | 96.675 s | 17.757 s |
| Estimated cost, off-peak | $0.017578 | $0.002300 |
| Estimated cost, peak | $0.035156 | $0.004600 |

See [Rig source](runs/rig/workspace/src/lib.rs), [Ax source](runs/ax/workspace/src/lib.rs), [Rig tool calls](runs/rig/tools.json), [Ax tool calls](runs/ax/tools.json), [Rig trace](runs/rig/trace.json), [Ax trace](runs/ax/trace.json), [Rig metrics](runs/rig/metrics.json), and [Ax metrics](runs/ax/metrics.json). Each `tools.json` records read/write/test calls and failures; the model saw test output and could revise code. The final `evaluate.md` is the actual host test output, not an LLM verdict. The saved workspaces contain model-authored code and tests; read them before running anything yourself.

**Accounting:** Rig reports aggregate usage from its agent runner. Ax's chat log reports uncached `prompt_tokens` separately from `cache_read_tokens`; the comparison adds them to match Rig's inclusive input count. A zero cache hit is inferred only if `total_tokens` equals prompt plus completion tokens. Costs use the [published DeepSeek Flash rates](https://api-docs.deepseek.com/quick_start/pricing/) for input cache misses, hits, and outputs; off-peak/peak rates differ. These are estimates, **not billed amounts**, and can change with model pricing and time of use.

**Interpretation:** Rig used additional turns and revised source after one failing test; Ax passed its own tests on the first attempt. Rig's output is larger and includes more tests; neither test count nor PASS measures unseen requirements or code quality. The follow-up [fixed-task comparison](benchmarks/SUMMARY.md) adds independent acceptance tests and repetitions. macOS `sandbox-exec` is defense-in-depth, not a robust multi-tenant security boundary.
