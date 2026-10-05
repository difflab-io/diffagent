# Tic-tac-toe run comparison

Both CLIs received the same input from [`examples/tic-tac-toe.md`](examples/tic-tac-toe.md) and executed the same [`diffagent.yaml`](diffagent.yaml) graph with `deepseek-chat`.

| Observation | Rig CLI | Ax CLI |
|---|---:|---:|
| Model-call path | Rig native DeepSeek client, async | Ax DeepSeek profile, blocking call offloaded to Tokio |
| Plan | 1,617 ms | 2,202 ms |
| Implement | 6,121 ms | 6,236 ms |
| LLM evaluation | 782 ms | 1,242 ms |
| Total model-call time (this one run) | 8,520 ms | 9,680 ms |
| Evaluator verdict | PASS | PASS |
| Revision triggered | No | No |

Read the actual [`Rig plan`](runs/rig/plan.md), [`Rig implementation`](runs/rig/implement.rs), [`Rig evaluation`](runs/rig/evaluate.md), and [`Rig trace`](runs/rig/trace.json); compare with the [`Ax plan`](runs/ax/plan.md), [`Ax implementation`](runs/ax/implement.rs), [`Ax evaluation`](runs/ax/evaluate.md), and [`Ax trace`](runs/ax/trace.json). The recorded `*.prompt.md` files show exactly what each call saw. The model responses are not deterministic; reruns will differ.

**Not a performance benchmark:** one sequential live run per backend, no control for network variance or generation length. **Not a coding-agent test:** no model tool calls, file edits, build, or unit tests were performed on either generated implementation. Both PASS verdicts are unverified model opinions. The FAIL → update edge is unit-tested, but these saved runs both took PASS → end.
