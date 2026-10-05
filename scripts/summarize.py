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
rows = []
for task in ("tic-tac-toe", "csv-parser"):
    for backend in ("rig", "ax"):
        for run in range(1, repetitions + 1):
            directory = results / task / backend / str(run)
            trace = read_json(directory, "trace.json") or []
            tools = read_json(directory, "tools.json") or []
            metrics = read_json(directory, "metrics.json") or {}
            evaluation = (directory / "evaluate.md").read_text() if (directory / "evaluate.md").exists() else ""
            exit_file = directory / "exit-code.txt"
            exit_code = int(exit_file.read_text()) if exit_file.exists() else None
            # cargo test reports library tests first, then the external integration suite.
            matches = re.findall(r"test result: ok\. (\d+) passed", evaluation)
            rows.append({
                "task": task, "backend": backend, "run": run,
                "exit_code": exit_code, "passed": exit_code == 0 and evaluation.startswith("PASS:"),
                "acceptance_tests": int(matches[1]) if len(matches) >= 2 else None,
                "elapsed_ms": sum(step["elapsed_ms"] for step in trace) if trace else None,
                "model_calls": metrics.get("usage", {}).get("model_calls"),
                "input_tokens": metrics.get("usage", {}).get("input_tokens"),
                "output_tokens": metrics.get("usage", {}).get("output_tokens"),
                "cached_input_tokens": metrics.get("usage", {}).get("cached_input_tokens"),
                "tool_calls": len(tools),
                "failed_test_calls": sum(e["name"] == "run_task" and not e["success"] for e in tools),
                "offpeak_usd_estimate": metrics.get("offpeak_usd_estimate"),
                "peak_usd_estimate": metrics.get("peak_usd_estimate"),
                "directory": str(directory.relative_to(root)),
            })

(root / "benchmarks/summary.json").write_text(json.dumps(rows, indent=2) + "\n")
lines = [
    "# Fixed-task comparison (DeepSeek Flash)", "",
    "Each backend receives the same public task specification and the same external acceptance tests. The model may edit only `src/lib.rs`; it cannot read or edit `tests/acceptance.rs` through its tools. Tests are copied into each run's workspace, so this is not adversarial isolation. The graph first allows in-agent tool calls, then independently runs `cargo test` for its pass/fail edge. These are small samples, not statistically sound benchmarks.", "",
    "| Task | Backend | Run | Result | Acceptance tests passed | Model calls | Input / cached / output tokens | Tool calls | Failed test calls | Stage time (s) | Est. USD off-peak / peak |",
    "|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|",
]
for r in rows:
    costs = ("unknown" if r["offpeak_usd_estimate"] is None else f'${r["offpeak_usd_estimate"]:.6f} / ${r["peak_usd_estimate"]:.6f}')
    seconds = "unknown" if r["elapsed_ms"] is None else f'{r["elapsed_ms"] / 1000:.1f}'
    relative = r["directory"].removeprefix("benchmarks/")
    lines.append(f'| [{r["task"]}]({relative}/) | {r["backend"]} | {r["run"]} | {"PASS" if r["passed"] else "FAIL"} | {fmt(r["acceptance_tests"])} | {fmt(r["model_calls"])} | {fmt(r["input_tokens"])} / {fmt(r["cached_input_tokens"])} / {fmt(r["output_tokens"])} | {r["tool_calls"]} | {r["failed_test_calls"]} | {seconds} | {costs} |')
lines += ["", "## Grouped outcome", "", "| Task | Backend | Passed / attempted | Mean observed time | Mean estimated off-peak cost |", "|---|---|---:|---:|---:|"]
for task in ("tic-tac-toe", "csv-parser"):
    for backend in ("rig", "ax"):
        subset = [r for r in rows if r["task"] == task and r["backend"] == backend]
        times = [r["elapsed_ms"] for r in subset if r["elapsed_ms"] is not None]
        costs = [r["offpeak_usd_estimate"] for r in subset if r["offpeak_usd_estimate"] is not None]
        lines.append(f'| {task} | {backend} | {sum(r["passed"] for r in subset)}/{len(subset)} | {mean(times)/1000:.1f}s' + ("" if len(times) == len(subset) else " (partial)") + f' | ${mean(costs):.6f}' + ("" if len(costs) == len(subset) else " (partial)") + ' |' if times and costs else f'| {task} | {backend} | {sum(r["passed"] for r in subset)}/{len(subset)} | unknown | unknown |')
lines += ["", "## Across both tasks", "", "| Backend | Passed / attempted | Mean stage time | Mean estimated off-peak cost |", "|---|---:|---:|---:|"]
for backend in ("rig", "ax"):
    subset = [r for r in rows if r["backend"] == backend]
    times = [r["elapsed_ms"] for r in subset if r["elapsed_ms"] is not None]
    costs = [r["offpeak_usd_estimate"] for r in subset if r["offpeak_usd_estimate"] is not None]
    lines.append(f'| {backend} | {sum(r["passed"] for r in subset)}/{len(subset)} | {mean(times)/1000:.1f}s | ${mean(costs):.6f} |' if len(times) == len(subset) and len(costs) == len(subset) else f'| {backend} | {sum(r["passed"] for r in subset)}/{len(subset)} | incomplete | incomplete |')
lines += [
    "", "**Caveats:** Each agent can author its own unit tests; the external acceptance suite is the common gate, not proof of all intended behavior. Token usage is backend-reported and can differ in accounting. Prices are estimates from [DeepSeek's published off-peak/peak rates](https://api-docs.deepseek.com/quick_start/pricing/), not billed charges. Generation length, cache state and network load vary. Review [diffagent.yaml](../diffagent.yaml), [the fixtures](fixtures/), individual `tools.json`, `trace.json`, `metrics.json`, and `workspace/src/lib.rs` before deciding.", "",
]
(root / "benchmarks/SUMMARY.md").write_text("\n".join(lines))
print(f"Wrote benchmarks/SUMMARY.md and summary.json for {len(rows)} attempted runs")
