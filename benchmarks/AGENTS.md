# Full-agent comparison (DeepSeek Flash)

Each backend receives the same public task specification and external acceptance tests. The full-agent comparison forces `implement=full`; the earlier Rig/Ax AxGen-tool-loop comparison is preserved in [SUMMARY.md](SUMMARY.md). Tests are copied into each run's workspace and cannot be read or edited through the model's tools, but this is not adversarial isolation. AxAgent uses a 16-step actor budget; Rig and ADK use 10 turns/iterations.

| Task | Backend | Run | Result | Acceptance tests passed | Model calls | Input / cached / output tokens | Tool calls | Failed test calls | Stage time (s) | Est. USD off-peak / peak |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|
| [tic-tac-toe](results/tic-tac-toe/rig/agent-1/) | rig | 1 | PASS | 4 | 7 | 21,552 / 17,536 / 5,593 | 7 | 1 | 24.2 | $0.004011 / $0.008022 |
| [tic-tac-toe](results/tic-tac-toe/rig/agent-2/) | rig | 2 | PASS | 4 | 5 | 8,813 / 5,632 / 3,261 | 5 | 0 | 16.5 | $0.002451 / $0.004901 |
| [tic-tac-toe](results/tic-tac-toe/ax/agent-1/) | ax | 1 | PASS | 4 | 7 | 18,274 / 13,440 / 5,273 | 4 | 0 | 89.5 | $0.003929 / $0.007858 |
| [tic-tac-toe](results/tic-tac-toe/ax/agent-2/) | ax | 2 | PASS | 4 | 6 | 15,593 / 10,240 / 11,888 | 4 | 0 | 46.5 | $0.007966 / $0.015933 |
| [tic-tac-toe](results/tic-tac-toe/adk/agent-1/) | adk | 1 | PASS | 4 | 9 | 79,314 / 30,848 / 14,421 | 9 | 1 | 52.2 | $0.016015 / $0.032030 |
| [tic-tac-toe](results/tic-tac-toe/adk/agent-2/) | adk | 2 | PASS | 4 | 7 | 34,798 / 3,200 / 11,477 | 7 | 0 | 45.3 | $0.011636 / $0.023271 |
| [csv-parser](results/csv-parser/rig/agent-1/) | rig | 1 | PASS | 4 | 9 | 57,197 / 52,864 / 12,962 | 9 | 1 | 51.9 | $0.008586 / $0.017171 |
| [csv-parser](results/csv-parser/rig/agent-2/) | rig | 2 | PASS | 4 | 5 | 9,113 / 6,272 / 3,843 | 5 | 0 | 19.5 | $0.002751 / $0.005502 |
| [csv-parser](results/csv-parser/ax/agent-1/) | ax | 1 | PASS | 4 | 7 | 20,254 / 13,696 / 19,290 | 6 | 1 | 197.4 | $0.012599 / $0.025198 |
| [csv-parser](results/csv-parser/ax/agent-2/) | ax | 2 | PASS | 4 | 8 | 21,259 / 15,360 / 20,852 | 4 | 0 | 142.9 | $0.013442 / $0.026884 |
| [csv-parser](results/csv-parser/adk/agent-1/) | adk | 1 | PASS | 4 | 6 | 21,130 / 4,352 / 7,281 | 6 | 0 | 31.2 | $0.006898 / $0.013797 |
| [csv-parser](results/csv-parser/adk/agent-2/) | adk | 2 | PASS | 4 | 8 | 42,747 / 19,456 / 11,662 | 8 | 0 | 46.9 | $0.010549 / $0.021098 |

**Pilot disclosure:** Before the saved four AxAgent runs, a pilot with a 10-step actor budget passed 1/4 runs; two exhausted the step budget and one emitted invalid structured JavaScript. The budget was raised to 16 and runtime instructions were clarified using these same tasks. The pilot was overwritten by the reruns, so this is tuning on the evaluation fixtures, not held-out evidence. The original pilot's total tokens and costs are unavailable; do not count the final 4/4 as an unbiased first-try rate.

## Grouped outcome

| Task | Backend | Passed / attempted | Mean observed time | Mean estimated off-peak cost |
|---|---|---:|---:|---:|
| tic-tac-toe | rig | 2/2 | 20.3s | $0.003231 |
| tic-tac-toe | ax | 2/2 | 68.0s | $0.005948 |
| tic-tac-toe | adk | 2/2 | 48.7s | $0.013825 |
| csv-parser | rig | 2/2 | 35.7s | $0.005668 |
| csv-parser | ax | 2/2 | 170.2s | $0.013020 |
| csv-parser | adk | 2/2 | 39.0s | $0.008724 |

## Across both tasks

| Backend | Passed / attempted | Mean stage time | Mean estimated off-peak cost |
|---|---:|---:|---:|
| rig | 4/4 | 28.0s | $0.004449 |
| ax | 4/4 | 119.1s | $0.009484 |
| adk | 4/4 | 43.9s | $0.011275 |

**Caveats:** The backends' full agents use different execution protocols; this compares end-to-end behavior under the same graph and external acceptance suite, not identical wire prompts or agent loops. Each agent can author its own unit tests; the external acceptance suite is not proof of all intended behavior. Failed model stages can incur calls not represented in saved aggregate metrics; their total time, tokens and cost are shown as unknown, not zero. Token usage is backend-reported and can differ in accounting. Prices are estimates from [DeepSeek's published off-peak/peak rates](https://api-docs.deepseek.com/quick_start/pricing/), not billed charges. Generation length, cache state, run order and network load vary. Review each run's `graph.yaml`, `agent-overrides.json`, `tools.json`, `trace.json`, `metrics.json`, and `workspace/src/lib.rs` before deciding.
