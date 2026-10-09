#!/usr/bin/env bash
set -u

: "${DEEPSEEK_API_KEY:?Set DEEPSEEK_API_KEY before running the benchmark}"
repetitions="${1:-2}"
if ! [[ "$repetitions" =~ ^[1-9][0-9]*$ ]] || (( repetitions > 5 )); then
  echo 'Usage: scripts/benchmark.sh [repetitions 1..5]' >&2
  exit 2
fi

cd "$(dirname "$0")/.." || exit 1
cargo build --workspace || exit $?
failures=0
for task in tic-tac-toe csv-parser; do
  for i in $(seq 1 "$repetitions"); do
    for backend in rig ax adk; do
      echo "== $task / $backend / full agent run $i =="
      cargo run --quiet -p "diffagent-$backend" -- \
        --fixture "$task" --run-id "agent-$i" --agent implement=full \
        --prompt-file "benchmarks/fixtures/$task/prompt.md"
      status=$?
      mkdir -p "benchmarks/results/$task/$backend/agent-$i"
      printf '%s\n' "$status" > "benchmarks/results/$task/$backend/agent-$i/exit-code.txt"
      echo "exit=$status"
      if (( status != 0 )); then failures=$((failures + 1)); fi
    done
  done
done
python3 scripts/summarize.py "$repetitions" --agents || exit $?
if (( failures != 0 )); then
  echo "$failures benchmark runs failed; see benchmarks/AGENTS.md" >&2
  exit 1
fi
