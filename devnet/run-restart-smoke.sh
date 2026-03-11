#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
config_path="${repo_root}/docs/config-examples/service-host-node.json"
state_dir="${repo_root}/docs/config-examples/.overlay-runtime/service-host-node"
tmpdir="$(mktemp -d)"
first_run_log="${tmpdir}/restart-run-1.log"
second_run_log="${tmpdir}/restart-run-2.log"
status_output="${tmpdir}/runtime-status.json"
node_pid=""

cleanup() {
  status=$?
  if [[ $status -ne 0 ]]; then
    for log in "$first_run_log" "$second_run_log"; do
      if [[ -f "$log" ]]; then
        echo "--- $(basename "$log") ---" >&2
        cat "$log" >&2
      fi
    done
    if [[ -f "$status_output" ]]; then
      echo "--- runtime-status.json ---" >&2
      cat "$status_output" >&2
    fi
  fi
  if [[ -n "${node_pid}" ]] && kill -0 "${node_pid}" 2>/dev/null; then
    kill -TERM "${node_pid}" 2>/dev/null || true
    wait "${node_pid}" 2>/dev/null || true
  fi
  rm -rf "$tmpdir"
  exit $status
}
trap cleanup EXIT

cd "${repo_root}"
rm -rf "${state_dir}"
TMPDIR=/tmp cargo build -p overlay-cli >/dev/null
overlay_cli="target/debug/overlay-cli"

"${overlay_cli}" run \
  --config "${config_path}" \
  --tick-ms 25 \
  --status-every 1 \
  >"${first_run_log}" 2>&1 &
node_pid="$!"

for _ in $(seq 1 200); do
  if "${overlay_cli}" status --config "${config_path}" >"${status_output}" 2>/dev/null; then
    if grep -q '"clean_shutdown":false' "${status_output}"; then
      break
    fi
  fi
  if ! kill -0 "${node_pid}" 2>/dev/null; then
    cat "${first_run_log}" >&2
    echo "restart smoke: first run exited before status became available" >&2
    exit 1
  fi
  sleep 0.05
done

kill -TERM "${node_pid}"
wait "${node_pid}"
node_pid=""

"${overlay_cli}" status --config "${config_path}" >"${status_output}"
grep -q '"shutdown_reason":"signal_terminate"' "${status_output}"
grep -q '"clean_shutdown":true' "${status_output}"
grep -q '"startup_count":1' "${status_output}"

"${overlay_cli}" run \
  --config "${config_path}" \
  --max-ticks 0 \
  --status-every 1 \
  >"${second_run_log}" 2>&1

"${overlay_cli}" status --config "${config_path}" >"${status_output}"
grep -q '"startup_count":2' "${status_output}"
grep -q '"previous_shutdown_clean":true' "${status_output}"
grep -q '"clean_shutdown":true' "${status_output}"

cat "${status_output}"
