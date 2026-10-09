# Provisional choice after the three-backend POC

Keep the diffagent graph and tool policy in `diffagent-core`. Use Rig as the first default for small coding tasks. Keep Ax and ADK-Rust as separate adapters. This is a provisional choice, not a production benchmark.

The [full-agent runs](AGENTS.md) passed all four external acceptance runs per backend. Rig averaged 28.0 seconds and $0.004449 estimated off-peak per run. AxAgent averaged 119.1 seconds and $0.009484. ADK averaged 43.9 seconds and $0.011275. The AxAgent pilot passed one of four runs before its actor budget and instructions were adjusted on these same tasks. The saved final runs are not held-out evidence of reliability.

The earlier [AxGen tool-loop runs](SUMMARY.md) were smaller and faster, but they did not exercise AxAgent. The SDKs also implement `full` differently. Rig runs a direct async agent loop. AxAgent runs a staged code-and-tool runtime. ADK uses a session-backed LlmAgent. Their native flow and checkpoint systems did not run the graph in this POC.

If prompt optimization is central, test AxGEPA on a larger training set and score it on held-out tasks. If durable workflow recovery is central, test ADK graph checkpoints and Rig's resumable agent state. Before shipping, test multi-file work, cancellation, streaming events, provider changes, durable chat, and a cross-platform execution boundary. The macOS test sandbox does not make generated code safe for hostile use.
