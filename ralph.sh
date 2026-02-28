#!/usr/bin/env bash
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TODO_FILE="$ROOT_DIR/TODO.md"
AGENTS_FILE="$ROOT_DIR/AGENTS.md"
LOG_FILE="${RALPH_LOG_FILE:-$ROOT_DIR/ralph.log}"
SLEEP_SECONDS="${RALPH_SLEEP_SECONDS:-5}"
MAX_ITERS="${RALPH_MAX_ITERS:-0}"

if ! command -v codex >/dev/null 2>&1; then
  echo "codex CLI is not installed or not on PATH" >&2
  exit 1
fi

if [[ ! -f "$TODO_FILE" ]]; then
  echo "Missing TODO file: $TODO_FILE" >&2
  exit 1
fi

if [[ ! -f "$AGENTS_FILE" ]]; then
  echo "Missing AGENTS file: $AGENTS_FILE" >&2
  exit 1
fi

touch "$LOG_FILE"

timestamp() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

pick_random_todo() {
  local open_tasks all_tasks

  mapfile -t open_tasks < <(grep -E '^[[:space:]]*- \[ \]' "$TODO_FILE" | sed -E 's/^[[:space:]]*- \[[ xX]\][[:space:]]*//')
  if ((${#open_tasks[@]} > 0)); then
    printf '%s\n' "${open_tasks[@]}" | shuf -n 1
    return
  fi

  mapfile -t all_tasks < <(grep -E '^[[:space:]]*- \[[xX ]\]' "$TODO_FILE" | sed -E 's/^[[:space:]]*- \[[ xX]\][[:space:]]*//')
  if ((${#all_tasks[@]} > 0)); then
    printf '%s\n' "${all_tasks[@]}" | shuf -n 1
    return
  fi

  printf 'General hardening and maintenance task\n'
}

build_prompt() {
  local seed_task="$1"

  cat <<PROMPT
Read AGENTS.md and TODO.md first.

Task seed selected by ralph.sh: "$seed_task"

Execution rules:
1. Pick a random TODO item (prefer unchecked; if all are checked, pick one completed item and improve/harden that area).
2. Implement the work directly in this repository.
3. Add or improve tests with strong edge-case coverage. Ensure tests are exhaustive for the changed behavior.
4. Run full project checks (fmt, check, test, clippy, plus any feature-specific checks needed).
5. If checks fail, fix and re-run until clean.
6. Update AGENTS.md learning notes if any important project knowledge is discovered.
7. Keep index.html milestone progress aligned with TODO.md whenever TODO milestone/checklist state changes.
8. Commit and push to origin/main.

At the end, provide a concise summary of what changed, what tests were run, and commit hash.
PROMPT
}

log_line() {
  local message="$1"
  printf '[%s] %s\n' "$(timestamp)" "$message" | tee -a "$LOG_FILE"
}

trap 'log_line "Received interrupt; exiting loop."; exit 0' INT TERM

iteration=1
while true; do
  if [[ "$MAX_ITERS" =~ ^[0-9]+$ ]] && ((MAX_ITERS > 0)) && ((iteration > MAX_ITERS)); then
    log_line "Reached RALPH_MAX_ITERS=$MAX_ITERS; exiting loop."
    break
  fi

  seed_task="$(pick_random_todo)"
  prompt="$(build_prompt "$seed_task")"

  log_line "Iteration $iteration starting. Seed task: $seed_task"
  codex exec \
    --dangerously-bypass-approvals-and-sandbox \
    -C "$ROOT_DIR" \
    "$prompt" 2>&1 | tee -a "$LOG_FILE"
  status=${PIPESTATUS[0]}
  log_line "Iteration $iteration finished with status $status"

  iteration=$((iteration + 1))
  sleep "$SLEEP_SECONDS"
done
