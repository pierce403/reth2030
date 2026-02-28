#!/usr/bin/env bash
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TODO_FILE="$ROOT_DIR/TODO.md"
AGENTS_FILE="$ROOT_DIR/AGENTS.md"
LOG_FILE="${RALPH_LOG_FILE:-$ROOT_DIR/ralph.log}"
SLEEP_SECONDS="${RALPH_SLEEP_SECONDS:-5}"
MAX_ITERS="${RALPH_MAX_ITERS:-0}"
MAX_RUNTIME_SECONDS="${RALPH_MAX_RUNTIME_SECONDS:-7200}"
SMALL_TASK_MAX_WORDS="${RALPH_SMALL_TASK_MAX_WORDS:-12}"

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

if ! [[ "$MAX_RUNTIME_SECONDS" =~ ^[0-9]+$ ]]; then
  echo "RALPH_MAX_RUNTIME_SECONDS must be a non-negative integer" >&2
  exit 1
fi

if ! [[ "$SMALL_TASK_MAX_WORDS" =~ ^[1-9][0-9]*$ ]]; then
  echo "RALPH_SMALL_TASK_MAX_WORDS must be a positive integer" >&2
  exit 1
fi

touch "$LOG_FILE"

timestamp() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

pick_random_todo() {
  local open_tasks all_tasks candidate_tasks small_tasks task

  mapfile -t open_tasks < <(grep -E '^[[:space:]]*- \[ \]' "$TODO_FILE" | sed -E 's/^[[:space:]]*- \[[ xX]\][[:space:]]*//')
  if ((${#open_tasks[@]} > 0)); then
    candidate_tasks=("${open_tasks[@]}")
  else
    mapfile -t all_tasks < <(grep -E '^[[:space:]]*- \[[xX ]\]' "$TODO_FILE" | sed -E 's/^[[:space:]]*- \[[ xX]\][[:space:]]*//')
    if ((${#all_tasks[@]} > 0)); then
      candidate_tasks=("${all_tasks[@]}")
    else
      printf 'small\tGeneral hardening and maintenance task\n'
      return
    fi
  fi

  small_tasks=()
  for task in "${candidate_tasks[@]}"; do
    if is_small_task "$task"; then
      small_tasks+=("$task")
    fi
  done

  if ((${#small_tasks[@]} > 0)); then
    printf 'small\t%s\n' "$(printf '%s\n' "${small_tasks[@]}" | shuf -n 1)"
    return
  fi

  printf 'big\t%s\n' "$(printf '%s\n' "${candidate_tasks[@]}" | shuf -n 1)"
}

is_small_task() {
  local task="$1"
  local words
  local task_lower

  task_lower="$(printf '%s' "$task" | tr '[:upper:]' '[:lower:]')"
  words="$(wc -w <<< "$task_lower")"
  if ((words > SMALL_TASK_MAX_WORDS)); then
    return 1
  fi

  if [[ "$task_lower" == *", "* ]] || [[ "$task_lower" == *";"* ]] || [[ "$task_lower" == *" and "* ]] || [[ "$task_lower" == *" plus "* ]] || [[ "$task_lower" == *"->"* ]]; then
    return 1
  fi

  return 0
}

build_prompt() {
  local seed_task="$1"
  local task_size_mode="$2"

  cat <<PROMPT
Read AGENTS.md and TODO.md first.

Task seed selected by ralph.sh (${task_size_mode} mode): "$seed_task"

Execution rules:
1. Prioritize a small TODO item (single focused deliverable; avoid compound scope).
2. Implement the work directly in this repository.
3. Add or improve tests with strong edge-case coverage. Ensure tests are exhaustive for the changed behavior.
4. Run full project checks (fmt, check, test, clippy, plus any feature-specific checks needed).
5. If checks fail, fix and re-run until clean.
6. Update AGENTS.md learning notes if any important project knowledge is discovered.
7. Keep index.html milestone progress aligned with TODO.md whenever TODO milestone/checklist state changes.
8. Commit and push to origin/main.

PROMPT

  if [[ "$task_size_mode" == "big" ]]; then
    cat <<PROMPT
Large-task handling rule:
- All current TODO candidates appear too big.
- Before coding, break the seeded task into 5-7 concrete unchecked subtasks in TODO.md.
- Make each subtask independently testable and small enough for one iteration.
- Complete one of the newly created subtasks in this iteration.

PROMPT
  fi

  cat <<PROMPT
At the end, provide a concise summary of what changed, what tests were run, and commit hash.
PROMPT
}

log_line() {
  local message="$1"
  printf '[%s] %s\n' "$(timestamp)" "$message" | tee -a "$LOG_FILE"
}

trap 'log_line "Received interrupt; exiting loop."; exit 0' INT TERM

TIMEOUT_CMD=""
if command -v timeout >/dev/null 2>&1; then
  TIMEOUT_CMD="timeout"
elif command -v gtimeout >/dev/null 2>&1; then
  TIMEOUT_CMD="gtimeout"
fi

start_epoch="$(date +%s)"

iteration=1
while true; do
  now_epoch="$(date +%s)"
  elapsed="$((now_epoch - start_epoch))"

  if ((MAX_RUNTIME_SECONDS > 0)) && ((elapsed >= MAX_RUNTIME_SECONDS)); then
    log_line "Reached RALPH_MAX_RUNTIME_SECONDS=$MAX_RUNTIME_SECONDS; exiting loop."
    break
  fi

  if [[ "$MAX_ITERS" =~ ^[0-9]+$ ]] && ((MAX_ITERS > 0)) && ((iteration > MAX_ITERS)); then
    log_line "Reached RALPH_MAX_ITERS=$MAX_ITERS; exiting loop."
    break
  fi

  task_selection="$(pick_random_todo)"
  if [[ "$task_selection" == *$'\t'* ]]; then
    task_size_mode="${task_selection%%$'\t'*}"
    seed_task="${task_selection#*$'\t'}"
  else
    task_size_mode="small"
    seed_task="$task_selection"
  fi

  prompt="$(build_prompt "$seed_task" "$task_size_mode")"

  log_line "Iteration $iteration starting. Mode=$task_size_mode Seed task: $seed_task"

  if ((MAX_RUNTIME_SECONDS > 0)); then
    now_epoch="$(date +%s)"
    remaining="$((MAX_RUNTIME_SECONDS - (now_epoch - start_epoch)))"
    if ((remaining <= 0)); then
      log_line "Reached RALPH_MAX_RUNTIME_SECONDS=$MAX_RUNTIME_SECONDS before launching iteration; exiting loop."
      break
    fi
  else
    remaining=0
  fi

  if ((MAX_RUNTIME_SECONDS > 0)) && [[ -n "$TIMEOUT_CMD" ]]; then
    "$TIMEOUT_CMD" --foreground "${remaining}s" \
      codex exec \
      --dangerously-bypass-approvals-and-sandbox \
      -C "$ROOT_DIR" \
      "$prompt" 2>&1 | tee -a "$LOG_FILE"
    status=${PIPESTATUS[0]}
    if ((status == 124)); then
      log_line "Iteration $iteration hit remaining runtime budget (${remaining}s) and was terminated."
      break
    fi
  else
    if ((MAX_RUNTIME_SECONDS > 0)) && [[ -z "$TIMEOUT_CMD" ]]; then
      log_line "No timeout command found; runtime limit will apply between iterations only."
    fi
    codex exec \
      --dangerously-bypass-approvals-and-sandbox \
      -C "$ROOT_DIR" \
      "$prompt" 2>&1 | tee -a "$LOG_FILE"
    status=${PIPESTATUS[0]}
  fi

  log_line "Iteration $iteration finished with status $status"

  iteration=$((iteration + 1))
  sleep "$SLEEP_SECONDS"
done
