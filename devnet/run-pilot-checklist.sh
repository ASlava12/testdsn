#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
cd "${repo_root}"

tmpdir="$(mktemp -d)"
bootstrap_a_log="${tmpdir}/pilot-bootstrap-a.log"
bootstrap_b_log="${tmpdir}/pilot-bootstrap-b.log"
bootstrap_relay_log="${tmpdir}/pilot-bootstrap-relay.log"
baseline_log="${tmpdir}/pilot-baseline.log"
node_down_log="${tmpdir}/pilot-node-down.log"
relay_fault_log="${tmpdir}/pilot-relay-fault.log"
bootstrap_fault_log="${tmpdir}/pilot-bootstrap-fault.log"
restart_run_1_log="${tmpdir}/pilot-restart-run-1.log"
restart_run_2_log="${tmpdir}/pilot-restart-run-2.log"
restart_status_output="${tmpdir}/pilot-restart-status.json"
bootstrap_a_pid=""
bootstrap_b_pid=""
bootstrap_relay_pid=""
restart_pid=""

cleanup() {
  status=$?
  if [[ $status -ne 0 ]]; then
    for log in \
      "${baseline_log}" \
      "${node_down_log}" \
      "${relay_fault_log}" \
      "${bootstrap_fault_log}" \
      "${restart_run_1_log}" \
      "${restart_run_2_log}" \
      "${bootstrap_a_log}" \
      "${bootstrap_b_log}" \
      "${bootstrap_relay_log}"; do
      if [[ -f "${log}" ]]; then
        echo "--- $(basename "${log}") ---" >&2
        cat "${log}" >&2
      fi
    done
    if [[ -f "${restart_status_output}" ]]; then
      echo "--- $(basename "${restart_status_output}") ---" >&2
      cat "${restart_status_output}" >&2
    fi
  fi
  if [[ -n "${restart_pid}" ]] && kill -0 "${restart_pid}" 2>/dev/null; then
    kill -TERM "${restart_pid}" 2>/dev/null || true
    wait "${restart_pid}" 2>/dev/null || true
  fi
  for pid_var in bootstrap_a_pid bootstrap_b_pid bootstrap_relay_pid; do
    pid="${!pid_var}"
    if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
      kill "${pid}" 2>/dev/null || true
      wait "${pid}" 2>/dev/null || true
    fi
  done
  rm -rf "${tmpdir}"
  exit "${status}"
}
trap cleanup EXIT

TMPDIR=/tmp cargo build -p overlay-cli >/dev/null
overlay_cli="target/debug/overlay-cli"

start_bootstrap_server() {
  local bind_addr="$1"
  local bootstrap_file="$2"
  local log_file="$3"
  "${overlay_cli}" bootstrap-serve \
    --bind "${bind_addr}" \
    --bootstrap-file "${bootstrap_file}" \
    >"${log_file}" 2>&1 &
  local pid="$!"
  for _ in $(seq 1 200); do
    if grep -q '"step":"bootstrap_server_listen"' "${log_file}"; then
      printf '%s\n' "${pid}"
      return 0
    fi
    if ! kill -0 "${pid}" 2>/dev/null; then
      cat "${log_file}" >&2
      echo "pilot checklist: bootstrap server ${bind_addr} exited before startup" >&2
      exit 1
    fi
    sleep 0.05
  done
  cat "${log_file}" >&2
  echo "pilot checklist: bootstrap server ${bind_addr} did not report readiness" >&2
  exit 1
}

stop_bootstrap_server() {
  local pid="$1"
  if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
    kill "${pid}" 2>/dev/null || true
    wait "${pid}" 2>/dev/null || true
  fi
}

require_step() {
  local log_file="$1"
  local step="$2"
  grep -q "\"step\":\"${step}\"" "${log_file}"
}

extract_numeric_field() {
  local log_file="$1"
  local field="$2"
  sed -n "s/.*\"${field}\":\\([0-9][0-9]*\\).*/\\1/p" "${log_file}" | head -n 1
}

emit_scenario_output() {
  local scenario="$1"
  local result="$2"
  local elapsed_ms="$3"
  local log_file="$4"
  echo "{\"step\":\"pilot_scenario\",\"scenario\":\"${scenario}\",\"result\":\"${result}\",\"elapsed_ms\":${elapsed_ms}}"
  cat "${log_file}"
}

run_restart_check() {
  local config_path="${repo_root}/devnet/pilot/localhost/configs/node-b.json"
  local state_dir="${repo_root}/devnet/pilot/localhost/configs/.overlay-runtime/node-b"

  rm -rf "${state_dir}"

  "${overlay_cli}" run \
    --config "${config_path}" \
    --tick-ms 25 \
    --status-every 1 \
    >"${restart_run_1_log}" 2>&1 &
  restart_pid="$!"

  for _ in $(seq 1 200); do
    if "${overlay_cli}" status --config "${config_path}" >"${restart_status_output}" 2>/dev/null; then
      if grep -q '"clean_shutdown":false' "${restart_status_output}"; then
        break
      fi
    fi
    if ! kill -0 "${restart_pid}" 2>/dev/null; then
      cat "${restart_run_1_log}" >&2
      echo "pilot checklist: restart check first run exited before status became available" >&2
      exit 1
    fi
    sleep 0.05
  done

  kill -TERM "${restart_pid}"
  wait "${restart_pid}"
  restart_pid=""

  "${overlay_cli}" status --config "${config_path}" >"${restart_status_output}"
  grep -q '"shutdown_reason":"signal_terminate"' "${restart_status_output}"
  grep -q '"clean_shutdown":true' "${restart_status_output}"
  grep -q '"startup_count":1' "${restart_status_output}"

  "${overlay_cli}" run \
    --config "${config_path}" \
    --max-ticks 0 \
    --status-every 1 \
    >"${restart_run_2_log}" 2>&1

  "${overlay_cli}" status --config "${config_path}" >"${restart_status_output}"
  grep -q '"startup_count":2' "${restart_status_output}"
  grep -q '"previous_shutdown_clean":true' "${restart_status_output}"
  grep -q '"clean_shutdown":true' "${restart_status_output}"
}

bootstrap_a_pid="$(start_bootstrap_server "127.0.0.1:4301" "devnet/pilot/localhost/bootstrap/node-foundation.json" "${bootstrap_a_log}")"
bootstrap_b_pid="$(start_bootstrap_server "127.0.0.1:4302" "devnet/pilot/localhost/bootstrap/node-a-seed.json" "${bootstrap_b_log}")"
bootstrap_relay_pid="$(start_bootstrap_server "127.0.0.1:4303" "devnet/pilot/localhost/bootstrap/node-ab-seed.json" "${bootstrap_relay_log}")"

baseline_started_ms="$(date +%s%3N)"
"${overlay_cli}" smoke --devnet-dir devnet/pilot/localhost >"${baseline_log}" 2>&1
baseline_elapsed_ms="$(( $(date +%s%3N) - baseline_started_ms ))"
require_step "${baseline_log}" "startup"
require_step "${baseline_log}" "session_established"
require_step "${baseline_log}" "publish_presence"
require_step "${baseline_log}" "lookup_node"
require_step "${baseline_log}" "open_service"
require_step "${baseline_log}" "relay_fallback_planned"
require_step "${baseline_log}" "relay_fallback_bound"
require_step "${baseline_log}" "smoke_complete"

node_down_started_ms="$(date +%s%3N)"
"${overlay_cli}" smoke --devnet-dir devnet/pilot/localhost --fault node-c-down >"${node_down_log}" 2>&1
node_down_elapsed_ms="$(( $(date +%s%3N) - node_down_started_ms ))"
require_step "${node_down_log}" "fault_injected"
grep -q '"fault":"node-c-down"' "${node_down_log}"
require_step "${node_down_log}" "smoke_complete"

relay_fault_started_ms="$(date +%s%3N)"
"${overlay_cli}" smoke --devnet-dir devnet/pilot/localhost --fault relay-unavailable >"${relay_fault_log}" 2>&1
relay_fault_elapsed_ms="$(( $(date +%s%3N) - relay_fault_started_ms ))"
require_step "${relay_fault_log}" "publish_presence"
require_step "${relay_fault_log}" "lookup_node"
require_step "${relay_fault_log}" "open_service"
require_step "${relay_fault_log}" "relay_fallback_planned"
require_step "${relay_fault_log}" "relay_fallback_unavailable"
require_step "${relay_fault_log}" "smoke_complete"

stop_bootstrap_server "${bootstrap_b_pid}"
bootstrap_b_pid=""

bootstrap_fault_started_ms="$(date +%s%3N)"
"${overlay_cli}" smoke --devnet-dir devnet/pilot/localhost >"${bootstrap_fault_log}" 2>&1
bootstrap_fault_elapsed_ms="$(( $(date +%s%3N) - bootstrap_fault_started_ms ))"
require_step "${bootstrap_fault_log}" "smoke_complete"

run_restart_check

emit_scenario_output "baseline" "ok" "${baseline_elapsed_ms}" "${baseline_log}"
emit_scenario_output "node-c-down" "ok" "${node_down_elapsed_ms}" "${node_down_log}"
emit_scenario_output "relay-unavailable" "expected_degraded" "${relay_fault_elapsed_ms}" "${relay_fault_log}"
emit_scenario_output "bootstrap-seed-unavailable" "ok" "${bootstrap_fault_elapsed_ms}" "${bootstrap_fault_log}"
echo '{"step":"pilot_scenario","scenario":"restart","result":"ok"}'
cat "${restart_status_output}"

baseline_lookup_latency_ms="$(extract_numeric_field "${baseline_log}" "lookup_latency_ms")"
node_down_lookup_latency_ms="$(extract_numeric_field "${node_down_log}" "lookup_latency_ms")"
bootstrap_fault_lookup_latency_ms="$(extract_numeric_field "${bootstrap_fault_log}" "lookup_latency_ms")"
baseline_relay_usage_bytes="$(extract_numeric_field "${baseline_log}" "relay_usage_bytes")"

echo "{\"step\":\"pilot_checklist_complete\",\"topology\":\"pilot-5-host\",\"baseline\":\"ok\",\"node_down\":\"ok\",\"relay_unavailable\":\"expected_degraded\",\"bootstrap_seed_unavailable\":\"ok\",\"restart\":\"ok\",\"baseline_lookup_latency_ms\":${baseline_lookup_latency_ms:-0},\"node_down_lookup_latency_ms\":${node_down_lookup_latency_ms:-0},\"bootstrap_seed_unavailable_lookup_latency_ms\":${bootstrap_fault_lookup_latency_ms:-0},\"baseline_relay_usage_bytes\":${baseline_relay_usage_bytes:-0}}"
