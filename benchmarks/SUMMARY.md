# Fixed-task comparison (DeepSeek Flash)

Each backend receives the same public task specification and the same external acceptance tests. The model may edit only `src/lib.rs`; it cannot read or edit `tests/acceptance.rs` through its tools. Tests are copied into each run's workspace, so this is not adversarial isolation. The graph first allows in-agent tool calls, then independently runs `cargo test` for its pass/fail edge. These are small samples, not statistically sound benchmarks.

| Task | Backend | Run | Result | Acceptance tests passed | Model calls | Input / cached / output tokens | Tool calls | Failed test calls | Stage time (s) | Est. USD off-peak / peak |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|
| [tic-tac-toe](results/tic-tac-toe/rig/1/) | rig | 1 | PASS | 4 | 5 | 8,853 / 5,504 / 3,279 | 5 | 0 | 16.5 | $0.002486 / $0.004973 |
| [tic-tac-toe](results/tic-tac-toe/rig/2/) | rig | 2 | PASS | 4 | 8 | 39,179 / 34,688 / 10,404 | 8 | 1 | 41.9 | $0.007020 / $0.014040 |
| [tic-tac-toe](results/tic-tac-toe/ax/1/) | ax | 1 | PASS | 4 | 7 | 22,080 / 19,712 / 5,163 | 7 | 0 | 25.0 | $0.003512 / $0.007024 |
| [tic-tac-toe](results/tic-tac-toe/ax/2/) | ax | 2 | PASS | 4 | 5 | 10,941 / 9,728 / 3,886 | 5 | 0 | 19.8 | $0.002543 / $0.005085 |
| [csv-parser](results/csv-parser/rig/1/) | rig | 1 | PASS | 4 | 5 | 11,970 / 8,704 / 4,705 | 5 | 0 | 22.1 | $0.003339 / $0.006678 |
| [csv-parser](results/csv-parser/rig/2/) | rig | 2 | PASS | 4 | 6 | 19,224 / 16,128 / 6,603 | 6 | 0 | 28.2 | $0.004475 / $0.008949 |
| [csv-parser](results/csv-parser/ax/1/) | ax | 1 | PASS | 4 | 5 | 12,507 / 11,136 / 5,187 | 5 | 0 | 25.4 | $0.003351 / $0.006703 |
| [csv-parser](results/csv-parser/ax/2/) | ax | 2 | PASS | 4 | 5 | 10,969 / 9,600 / 4,109 | 5 | 0 | 20.9 | $0.002700 / $0.005399 |

## Grouped outcome

| Task | Backend | Passed / attempted | Mean observed time | Mean estimated off-peak cost |
|---|---|---:|---:|---:|
| tic-tac-toe | rig | 2/2 | 29.2s | $0.004753 |
| tic-tac-toe | ax | 2/2 | 22.4s | $0.003027 |
| csv-parser | rig | 2/2 | 25.2s | $0.003907 |
| csv-parser | ax | 2/2 | 23.1s | $0.003025 |

## Across both tasks

| Backend | Passed / attempted | Mean stage time | Mean estimated off-peak cost |
|---|---:|---:|---:|
| rig | 4/4 | 27.2s | $0.004330 |
| ax | 4/4 | 22.8s | $0.003026 |

**Caveats:** Each agent can author its own unit tests; the external acceptance suite is the common gate, not proof of all intended behavior. Token usage is backend-reported and can differ in accounting. Prices are estimates from [DeepSeek's published off-peak/peak rates](https://api-docs.deepseek.com/quick_start/pricing/), not billed charges. Generation length, cache state and network load vary. Review [diffagent.yaml](../diffagent.yaml), [the fixtures](fixtures/), individual `tools.json`, `trace.json`, `metrics.json`, and `workspace/src/lib.rs` before deciding.
