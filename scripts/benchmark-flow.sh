#!/usr/bin/env bash
# Flow comparison. Never overwrites the historical benchmark results.
set -u
: "${DEEPSEEK_API_KEY:?Set DEEPSEEK_API_KEY before running the comparison}"
repetitions="${1:-2}"
backend_filter="${2:-all}"
task_filter="${3:-all}"
if (( $# > 3 )); then
  echo 'Usage: scripts/benchmark-flow.sh [repetitions 1..5] [rig|ax|adk|all] [tic-tac-toe|csv-parser|all]' >&2
  exit 2
fi
case "$backend_filter" in rig|ax|adk|all) ;; *) echo "Unknown backend: $backend_filter" >&2; exit 2 ;; esac
case "$task_filter" in tic-tac-toe|csv-parser|all) ;; *) echo "Unknown fixture: $task_filter" >&2; exit 2 ;; esac
run_set="${DIFFAGENT_RUN_SET:-current}"
if ! [[ "$run_set" =~ ^[A-Za-z0-9_-]+$ ]]; then
  echo 'DIFFAGENT_RUN_SET must contain only letters, digits, underscores, or hyphens' >&2
  exit 2
fi
flow_timeout="${DIFFAGENT_FLOW_TIMEOUT_SECS:-240}"
if ! [[ "$flow_timeout" =~ ^[1-9][0-9]*$ ]]; then
  echo 'DIFFAGENT_FLOW_TIMEOUT_SECS must be a positive integer' >&2
  exit 2
fi
if ! [[ "$repetitions" =~ ^[1-9][0-9]*$ ]] || (( repetitions > 5 )); then
  echo 'Usage: scripts/benchmark-flow.sh [repetitions 1..5] [rig|ax|adk|all] [tic-tac-toe|csv-parser|all]' >&2
  exit 2
fi
cd "$(dirname "$0")/.." || exit 1
root="$PWD"
packages=(-p diffagent-core)
for backend in rig ax adk; do
  if [[ "$backend_filter" == all || "$backend_filter" == "$backend" ]]; then
    packages+=(-p "diffagent-$backend")
  fi
done
cargo build "${packages[@]}" || exit $?
failures=0
for task in tic-tac-toe csv-parser; do
  if [[ "$task_filter" != all && "$task_filter" != "$task" ]]; then continue; fi
  for i in $(seq 1 "$repetitions"); do
    for backend in rig ax adk; do
      if [[ "$backend_filter" != all && "$backend_filter" != "$backend" ]]; then continue; fi
      run="$root/benchmarks/results-next/$run_set/$task/$backend/run-$i"
      if [[ -e "$run" ]]; then
        echo "Refusing to overwrite $run" >&2
        exit 2
      fi
      workspace="$run/workspace"
      mkdir -p "$workspace/src" "$workspace/tests" || exit 1
      printf '[package]\nname = "generated-example"\nversion = "0.1.0"\nedition = "2024"\n\n[workspace]\n' > "$workspace/Cargo.toml"
      printf '// Agent implementation starts here.\n' > "$workspace/src/lib.rs"
      # The flow may run tests, but its file tools cannot read or edit acceptance.rs.
      cp "$root/benchmarks/fixtures/$task/tests/acceptance.rs" "$workspace/tests/acceptance.rs"
      echo "== $task / $backend / run $i =="
      flow_started=$SECONDS
      python3 scripts/with-timeout.py "$flow_timeout" cargo run --quiet -p "diffagent-$backend" -- \
        flow implement --agent "$root/agent-benchmark.yaml" --workspace "$workspace" \
        --arg "request=$(<"$root/benchmarks/fixtures/$task/prompt.md")" \
        > "$run/flow-output.txt" 2> "$run/flow-error.txt"
      flow_status=$?
      printf '%s\n' "$((SECONDS - flow_started))" > "$run/flow-wall-seconds.txt"
      acceptance_started=$SECONDS
      cargo run --quiet -p diffagent-core --bin check-task -- \
        "$root/agent-benchmark.yaml" "$workspace" test \
        > "$run/acceptance-output.txt" 2> "$run/acceptance-error.txt"
      acceptance_status=$?
      printf '%s\n' "$((SECONDS - acceptance_started))" > "$run/acceptance-wall-seconds.txt"
      printf '%s\n' "$flow_status" > "$run/flow-exit-code.txt"
      printf '%s\n' "$acceptance_status" > "$run/acceptance-exit-code.txt"
      echo "flow=$flow_status acceptance=$acceptance_status artifacts=$run"
      if (( flow_status != 0 || acceptance_status != 0 )); then
        failures=$((failures + 1))
      fi
    done
  done
done
if (( failures != 0 )); then
  echo "$failures new-flow runs failed; see benchmarks/results-next" >&2
  exit 1
fi
