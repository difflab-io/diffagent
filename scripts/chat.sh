#!/usr/bin/env bash
# Start a chat in the directory from which this script was launched.
set -euo pipefail

: "${DEEPSEEK_API_KEY:?Set DEEPSEEK_API_KEY in your environment}"
backend="${1:-}"
case "$backend" in
  rig|ax|adk) ;;
  *) echo 'Usage: bash scripts/chat.sh rig|ax|adk' >&2; exit 2 ;;
esac

launch_workspace="$(pwd -P)"
cd "$(dirname "$0")/.."
root="$PWD"
if [[ -n "${DIFFAGENT_WORKSPACE:-}" ]]; then
  if [[ ! -d "$DIFFAGENT_WORKSPACE" ]]; then
    echo "Workspace does not exist: $DIFFAGENT_WORKSPACE" >&2
    exit 2
  fi
  workspace="$(cd "$DIFFAGENT_WORKSPACE" && pwd -P)"
else
  workspace="$launch_workspace"
fi
printf 'Chat workspace: %s\n' "$workspace" >&2
exec cargo run --quiet -p "diffagent-${backend}" -- chat \
  --agent "$root/agent.yaml" --workspace "$workspace"
