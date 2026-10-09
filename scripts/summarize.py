#!/usr/bin/env python3
"""Summarize saved benchmark artifacts; never guess missing token/cost data."""

import json
import re
import sys
from pathlib import Path
from statistics import mean


def read_json(directory: Path, name: str):
    try:
        return json.loads((directory / name).read_text())
    except FileNotFoundError:
        return None
    except json.JSONDecodeError as exc:
        print(f"Invalid {directory / name}: {exc}", file=sys.stderr)
        return None


def fmt(value):
    return "unknown" if value is None else f"{value:,}"


root = Path(__file__).resolve().parent.parent
results = root / "benchmarks/results"
repetitions = int(sys.argv[1]) if len(sys.argv) > 1 else 2
agents = len(sys.argv) > 2 and sys.argv[2] == "--agents"
backends = ("rig", "ax", "adk") if agents else ("rig", "ax")
rows = []
for task in ("tic-tac-toe", "csv-parser"):
    for backend in backends:
        for run in range(1, repetitions + 1):
            directory = results / task / backend / (f"agent-{run}" if agents else str(run))
            trace = read_json(directory, "trace.json") or []
            tools = read_json(directory, "tools.json") or []
            metrics = read_json(directory, "metrics.json") or {}
            evaluation = (directory / "evaluate.md").read_text() if (directory / "evaluate.md").exists() else ""
            exit_file = directory / "exit-code.txt"
            exit_code = int(exit_file.read_text()) if exit_file.exists() else None
            # cargo test reports library tests first, then the external integration suite.
            matches = re.findall(r"test result: ok\. (\d+) passed", evaluation)
            acceptance_tests = int(matches[1]) if len(matches) >= 2 else None
            implementation = next((step for step in trace if step["node"] == "implement"), {})
            complete = exit_code == 0 and bool(evaluation)
            error_file = directory / "implement.error.md"
            rows.append({
                "task": task, "backend": backend, "run": run,
                "exit_code": exit_code,
                "passed": exit_code == 0 and evaluation.startswith("PASS:") and acceptance_tests == 4 and (not agents or implementation.get("agent") == "full"),
                "implement_agent": implementation.get("agent"),
                "acceptance_tests": acceptance_tests,
                "elapsed_ms": sum(step["elapsed_ms"] for step in trace) if complete else None,
                "model_calls": metrics.get("usage", {}).get("model_calls") if complete else None,
                "input_tokens": metrics.get("usage", {}).get("input_tokens") if complete else None,
                "output_tokens": metrics.get("usage", {}).get("output_tokens") if complete else None,
                "cached_input_tokens": metrics.get("usage", {}).get("cached_input_tokens") if complete else None,
                "error": error_file.read_text()[:300] if error_file.exists() else None,
                "tool_calls": len(tools),
                "failed_test_calls": sum(e["name"] == "run_task" and not e["success"] for e in tools),
                "offpeak_usd_estimate": metrics.get("offpeak_usd_estimate") if complete else None,
                "peak_usd_estimate": metrics.get("peak_usd_estimate") if complete else None,
                "directory": str(directory.relative_to(root)),
            })

json_file = "agents.json" if agents else "summary.json"
markdown_file = "AGENTS.md" if agents else "SUMMARY.md"
(root / "benchmarks" / json_file).write_text(json.dumps(rows, indent=2) + "\n")
lines = [
    "# Full-agent comparison (DeepSeek Flash)" if agents else "# Fixed-task comparison (DeepSeek Flash)", "",
    "Each backend receives the same public task specification and external acceptance tests. The full-agent comparison forces `implement=full`; the earlier Rig/Ax AxGen-tool-loop comparison is preserved in [SUMMARY.md](SUMMARY.md). Tests are copied into each run's workspace and cannot be read or edited through the model's tools, but this is not adversarial isolation. AxAgent uses a 16-step actor budget; Rig and ADK use 10 turns/iterations." if agents else "Each backend receives the same public task specification and the same external acceptance tests. The model may edit only `src/lib.rs`; it cannot read or edit `tests/acceptance.rs` through its tools. Tests are copied into each run's workspace, so this is not adversarial isolation. The graph first allows in-agent tool calls, then independently runs `cargo test` for its pass/fail edge. These are small samples, not statistically sound benchmarks.", "",
    "| Task | Backend | Run | Result | Acceptance tests passed | Model calls | Input / cached / output tokens | Tool calls | Failed test calls | Stage time (s) | Est. USD off-peak / peak |",
    "|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|",
]
for r in rows:
    costs = ("unknown" if r["offpeak_usd_estimate"] is None else f'${r["offpeak_usd_estimate"]:.6f} / ${r["peak_usd_estimate"]:.6f}')
    seconds = "unknown" if r["elapsed_ms"] is None else f'{r["elapsed_ms"] / 1000:.1f}'
    relative = r["directory"].removeprefix("benchmarks/")
    lines.append(f'| [{r["task"]}]({relative}/) | {r["backend"]} | {r["run"]} | {"PASS" if r["passed"] else "FAIL"} | {fmt(r["acceptance_tests"])} | {fmt(r["model_calls"])} | {fmt(r["input_tokens"])} / {fmt(r["cached_input_tokens"])} / {fmt(r["output_tokens"])} | {r["tool_calls"]} | {r["failed_test_calls"]} | {seconds} | {costs} |')
if agents:
    lines += ["", "**Pilot disclosure:** Before the saved four AxAgent runs, a pilot with a 10-step actor budget passed 1/4 runs; two exhausted the step budget and one emitted invalid structured JavaScript. The budget was raised to 16 and runtime instructions were clarified using these same tasks. The pilot was overwritten by the reruns, so this is tuning on the evaluation fixtures, not held-out evidence. The original pilot's total tokens and costs are unavailable; do not count the final 4/4 as an unbiased first-try rate."]
lines += ["", "## Grouped outcome", "", "| Task | Backend | Passed / attempted | Mean observed time | Mean estimated off-peak cost |", "|---|---|---:|---:|---:|"]
for task in ("tic-tac-toe", "csv-parser"):
    for backend in backends:
        subset = [r for r in rows if r["task"] == task and r["backend"] == backend]
        times = [r["elapsed_ms"] for r in subset if r["elapsed_ms"] is not None]
        costs = [r["offpeak_usd_estimate"] for r in subset if r["offpeak_usd_estimate"] is not None]
        lines.append(f'| {task} | {backend} | {sum(r["passed"] for r in subset)}/{len(subset)} | {mean(times)/1000:.1f}s' + ("" if len(times) == len(subset) else " (partial)") + f' | ${mean(costs):.6f}' + ("" if len(costs) == len(subset) else " (partial)") + ' |' if times and costs else f'| {task} | {backend} | {sum(r["passed"] for r in subset)}/{len(subset)} | unknown | unknown |')
lines += ["", "## Across both tasks", "", "| Backend | Passed / attempted | Mean stage time | Mean estimated off-peak cost |", "|---|---:|---:|---:|"]
for backend in backends:
    subset = [r for r in rows if r["backend"] == backend]
    times = [r["elapsed_ms"] for r in subset if r["elapsed_ms"] is not None]
    costs = [r["offpeak_usd_estimate"] for r in subset if r["offpeak_usd_estimate"] is not None]
    lines.append(f'| {backend} | {sum(r["passed"] for r in subset)}/{len(subset)} | {mean(times)/1000:.1f}s | ${mean(costs):.6f} |' if len(times) == len(subset) and len(costs) == len(subset) else f'| {backend} | {sum(r["passed"] for r in subset)}/{len(subset)} | incomplete | incomplete |')
lines += [
    "", "**Caveats:** The backends' full agents use different execution protocols; this compares end-to-end behavior under the same graph and external acceptance suite, not identical wire prompts or agent loops. Each agent can author its own unit tests; the external acceptance suite is not proof of all intended behavior. Failed model stages can incur calls not represented in saved aggregate metrics; their total time, tokens and cost are shown as unknown, not zero. Token usage is backend-reported and can differ in accounting. Prices are estimates from [DeepSeek's published off-peak/peak rates](https://api-docs.deepseek.com/quick_start/pricing/), not billed charges. Generation length, cache state, run order and network load vary. Review each run's `graph.yaml`, `agent-overrides.json`, `tools.json`, `trace.json`, `metrics.json`, and `workspace/src/lib.rs` before deciding.", "",
]
(root / "benchmarks" / markdown_file).write_text("\n".join(lines))
print(f"Wrote benchmarks/{markdown_file} and {json_file} for {len(rows)} attempted runs")
