#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
cd "${repo_root}"

tmpdir="$(mktemp -d)"
bootstrap_a_log="${tmpdir}/pilot-bootstrap-a.log"
bootstrap_b_log="${tmpdir}/pilot-bootstrap-b.log"
bootstrap_relay_log="${tmpdir}/pilot-bootstrap-relay.log"
node_a_log="${tmpdir}/pilot-node-a.log"
node_b_log="${tmpdir}/pilot-node-b.log"
node_c_log="${tmpdir}/pilot-node-c.log"
relay_a_log="${tmpdir}/pilot-relay-a.log"
relay_b_log="${tmpdir}/pilot-relay-b.log"
baseline_publish_log="${tmpdir}/baseline-publish.log"
baseline_lookup_log="${tmpdir}/baseline-lookup.log"
baseline_service_log="${tmpdir}/baseline-service.log"
baseline_relay_a_log="${tmpdir}/baseline-relay-a.log"
baseline_relay_b_log="${tmpdir}/baseline-relay-b.log"
node_down_lookup_log="${tmpdir}/node-down-lookup.log"
node_down_service_log="${tmpdir}/node-down-service.log"
relay_fault_primary_log="${tmpdir}/relay-fault-primary.log"
relay_fault_alternate_log="${tmpdir}/relay-fault-alternate.log"
bootstrap_fault_restart_log="${tmpdir}/bootstrap-fault-restart.log"
service_restart_log="${tmpdir}/service-restart.log"
service_restart_status="${tmpdir}/service-restart-status.json"
tampered_bootstrap_log="${tmpdir}/tampered-bootstrap.log"
relay_a_status="${tmpdir}/relay-a-status.json"
relay_b_status="${tmpdir}/relay-b-status.json"
bootstrap_a_pid=""
bootstrap_b_pid=""
bootstrap_relay_pid=""
node_a_pid=""
node_b_pid=""
node_c_pid=""
relay_a_pid=""
relay_b_pid=""

cleanup() {
  status=$?
  if [[ $status -ne 0 ]]; then
    for log in \
      "${baseline_publish_log}" \
      "${baseline_lookup_log}" \
      "${baseline_service_log}" \
      "${baseline_relay_a_log}" \
      "${baseline_relay_b_log}" \
      "${node_down_lookup_log}" \
      "${node_down_service_log}" \
      "${relay_fault_primary_log}" \
      "${relay_fault_alternate_log}" \
      "${bootstrap_fault_restart_log}" \
      "${service_restart_log}" \
      "${service_restart_status}" \
      "${tampered_bootstrap_log}" \
      "${node_a_log}" \
      "${node_b_log}" \
      "${node_c_log}" \
      "${relay_a_log}" \
      "${relay_b_log}" \
      "${bootstrap_a_log}" \
      "${bootstrap_b_log}" \
      "${bootstrap_relay_log}" \
      "${relay_a_status}" \
      "${relay_b_status}"; do
      if [[ -f "${log}" ]]; then
        echo "--- $(basename "${log}") ---" >&2
        cat "${log}" >&2
      fi
    done
  fi
  for pid_var in node_a_pid node_b_pid node_c_pid relay_a_pid relay_b_pid bootstrap_a_pid bootstrap_b_pid bootstrap_relay_pid; do
    pid="${!pid_var}"
    if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
      kill -TERM "${pid}" 2>/dev/null || true
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
      echo "distributed pilot checklist: bootstrap server ${bind_addr} exited before startup" >&2
      exit 1
    fi
    sleep 0.05
  done
  cat "${log_file}" >&2
  echo "distributed pilot checklist: bootstrap server ${bind_addr} did not report readiness" >&2
  exit 1
}

start_node() {
  local config_path="$1"
  local log_file="$2"
  shift 2
  "${overlay_cli}" run \
    --config "${config_path}" \
    --tick-ms 25 \
    --max-ticks 900 \
    "$@" \
    >"${log_file}" 2>&1 &
  printf '%s\n' "$!"
}

status_probe_file() {
  local config_path="$1"
  printf '%s/%s.status.json\n' "${tmpdir}" "$(basename "${config_path}" .json)"
}

runtime_lock_file() {
  local config_path="$1"
  printf '%s/.overlay-runtime/%s/runtime.lock\n' \
    "$(dirname "${config_path}")" \
    "$(basename "${config_path}" .json)"
}

status_matches_pid() {
  local status_file="$1"
  local pid="$2"
  grep -q "\"pid\":${pid}" "${status_file}"
}

stop_process() {
  local pid="$1"
  if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
    kill -TERM "${pid}" 2>/dev/null || true
    wait "${pid}" 2>/dev/null || true
  fi
}

wait_for_lock_release() {
  local pid="$1"
  local config_path="$2"
  local description="$3"
  local lock_file
  lock_file="$(runtime_lock_file "${config_path}")"
  for _ in $(seq 1 240); do
    if ! kill -0 "${pid}" 2>/dev/null && [[ ! -f "${lock_file}" ]]; then
      return 0
    fi
    sleep 0.05
  done
  echo "distributed pilot checklist: ${description} did not release ${lock_file}" >&2
  exit 1
}

wait_for_runtime() {
  local pid="$1"
  local config_path="$2"
  local log_file="$3"
  local listener_pattern="${4:-}"
  local status_file
  status_file="$(status_probe_file "${config_path}")"
  for _ in $(seq 1 240); do
    if [[ -n "${listener_pattern}" ]] && ! grep -q "${listener_pattern}" "${log_file}"; then
      :
    elif "${overlay_cli}" status --config "${config_path}" >"${status_file}" 2>/dev/null \
      && status_matches_pid "${status_file}" "${pid}"; then
      return 0
    fi
    if ! kill -0 "${pid}" 2>/dev/null; then
      cat "${log_file}" >&2
      echo "distributed pilot checklist: runtime ${config_path} exited before status became available" >&2
      exit 1
    fi
    sleep 0.05
  done
  cat "${log_file}" >&2
  echo "distributed pilot checklist: runtime ${config_path} did not become ready" >&2
  exit 1
}

wait_for_status_pattern() {
  local pid="$1"
  local config_path="$2"
  local status_file="$3"
  local pattern="$4"
  local description="$5"
  for _ in $(seq 1 240); do
    if "${overlay_cli}" status --config "${config_path}" >"${status_file}" 2>/dev/null \
      && status_matches_pid "${status_file}" "${pid}" \
      && grep -q "${pattern}" "${status_file}"; then
      return 0
    fi
    if ! kill -0 "${pid}" 2>/dev/null; then
      echo "distributed pilot checklist: runtime ${config_path} exited before ${description}" >&2
      exit 1
    fi
    sleep 0.05
  done
  echo "distributed pilot checklist: runtime ${config_path} did not reach ${description}" >&2
  exit 1
}

extract_node_id() {
  local pid="$1"
  local config_path="$2"
  local status_file
  status_file="$(status_probe_file "${config_path}")"
  wait_for_status_pattern "${pid}" "${config_path}" "${status_file}" '"node_id":"' "fresh node status"
  sed -n 's/.*"node_id":"\([0-9a-f]\{64\}\)".*/\1/p' "${status_file}" | head -n 1
}

extract_numeric_field() {
  local log_file="$1"
  local field="$2"
  sed -n "s/.*\"${field}\":\\([0-9][0-9]*\\).*/\\1/p" "${log_file}" | head -n 1
}

extract_status_numeric_field() {
  local status_file="$1"
  local field="$2"
  sed -n "s/.*\"${field}\":\\([0-9][0-9]*\\).*/\\1/p" "${status_file}" | head -n 1
}

wait_for_startup_count_increment() {
  local pid="$1"
  local config_path="$2"
  local status_file="$3"
  local minimum_startup_count="$4"
  local description="$5"
  for _ in $(seq 1 240); do
    if "${overlay_cli}" status --config "${config_path}" >"${status_file}" 2>/dev/null \
      && status_matches_pid "${status_file}" "${pid}"; then
      local startup_count
      startup_count="$(extract_status_numeric_field "${status_file}" "startup_count")"
      if [[ -n "${startup_count}" ]] \
        && [[ "${startup_count}" -ge "${minimum_startup_count}" ]] \
        && grep -q '"previous_shutdown_clean":true' "${status_file}"; then
        return 0
      fi
    fi
    if ! kill -0 "${pid}" 2>/dev/null; then
      echo "distributed pilot checklist: runtime ${config_path} exited before ${description}" >&2
      exit 1
    fi
    sleep 0.05
  done
  echo "distributed pilot checklist: runtime ${config_path} did not reach ${description}" >&2
  exit 1
}

start_full_topology() {
  bootstrap_a_pid="$(start_bootstrap_server "127.0.0.1:4301" "devnet/pilot/localhost/bootstrap/node-foundation.json" "${bootstrap_a_log}")"
  bootstrap_b_pid="$(start_bootstrap_server "127.0.0.1:4302" "devnet/pilot/localhost/bootstrap/node-a-seed.json" "${bootstrap_b_log}")"
  bootstrap_relay_pid="$(start_bootstrap_server "127.0.0.1:4303" "devnet/pilot/localhost/bootstrap/node-ab-seed.json" "${bootstrap_relay_log}")"

  node_a_pid="$(start_node "devnet/pilot/localhost/configs/node-a.json" "${node_a_log}")"
  node_b_pid="$(start_node "devnet/pilot/localhost/configs/node-b.json" "${node_b_log}" --service devnet:terminal)"
  node_c_pid="$(start_node "devnet/pilot/localhost/configs/node-c.json" "${node_c_log}")"
  relay_a_pid="$(start_node "devnet/pilot/localhost/configs/node-relay.json" "${relay_a_log}")"
  relay_b_pid="$(start_node "devnet/pilot/localhost/configs/node-relay-b.json" "${relay_b_log}")"

  wait_for_runtime "${node_a_pid}" "devnet/pilot/localhost/configs/node-a.json" "${node_a_log}" '"event":"listen","result":"ok"'
  wait_for_runtime "${node_b_pid}" "devnet/pilot/localhost/configs/node-b.json" "${node_b_log}" '"event":"listen","result":"ok"'
  wait_for_runtime "${node_c_pid}" "devnet/pilot/localhost/configs/node-c.json" "${node_c_log}" '"event":"listen","result":"ok"'
  wait_for_runtime "${relay_a_pid}" "devnet/pilot/localhost/configs/node-relay.json" "${relay_a_log}" '"event":"listen","result":"ok"'
  wait_for_runtime "${relay_b_pid}" "devnet/pilot/localhost/configs/node-relay-b.json" "${relay_b_log}" '"event":"listen","result":"ok"'
}

node_a_id=""
node_b_id=""
relay_a_id=""
relay_b_id=""

run_baseline_flow() {
  node_a_id="$(extract_node_id "${node_a_pid}" "devnet/pilot/localhost/configs/node-a.json")"
  node_b_id="$(extract_node_id "${node_b_pid}" "devnet/pilot/localhost/configs/node-b.json")"
  relay_a_id="$(extract_node_id "${relay_a_pid}" "devnet/pilot/localhost/configs/node-relay.json")"
  relay_b_id="$(extract_node_id "${relay_b_pid}" "devnet/pilot/localhost/configs/node-relay-b.json")"

  "${overlay_cli}" publish \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4111 \
    --relay-ref "${relay_a_id}" \
    --relay-ref "${relay_b_id}" \
    --capability service-host \
    >"${baseline_publish_log}"

  "${overlay_cli}" lookup \
    --config devnet/pilot/localhost/configs/node-a.json \
    --target tcp://127.0.0.1:4111 \
    --node-id "${node_b_id}" \
    >"${baseline_lookup_log}"

  "${overlay_cli}" open-service \
    --config devnet/pilot/localhost/configs/node-a.json \
    --target tcp://127.0.0.1:4112 \
    --target-node-id "${node_b_id}" \
    --service-namespace devnet \
    --service-name terminal \
    >"${baseline_service_log}"

  echo "{\"step\":\"relay_fallback_planned\",\"client_node\":\"node-a\",\"target_node\":\"node-b\",\"relay_node\":\"node-relay\",\"relay_node_id\":\"${relay_a_id}\",\"alternate_relay_node_id\":\"${relay_b_id}\"}"
  "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4198 \
    --relay-node-id "${relay_a_id}" \
    --requester-node-id "${node_a_id}" \
    >"${baseline_relay_a_log}"

  echo "{\"step\":\"relay_fallback_planned\",\"client_node\":\"node-a\",\"target_node\":\"node-b\",\"relay_node\":\"node-relay-b\",\"relay_node_id\":\"${relay_b_id}\",\"alternate_relay_node_id\":\"${relay_a_id}\"}"
  "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4197 \
    --relay-node-id "${relay_b_id}" \
    --requester-node-id "${node_a_id}" \
    >"${baseline_relay_b_log}"

  wait_for_status_pattern \
    "${relay_a_pid}" \
    "devnet/pilot/localhost/configs/node-relay.json" \
    "${relay_a_status}" \
    '"active_tunnels":1' \
    'primary relay tunnel bind'
  wait_for_status_pattern \
    "${relay_b_pid}" \
    "devnet/pilot/localhost/configs/node-relay-b.json" \
    "${relay_b_status}" \
    '"active_tunnels":1' \
    'alternate relay tunnel bind'
}

run_baseline_flow_with_node_c_down() {
  stop_process "${node_c_pid}"
  node_c_pid=""
  "${overlay_cli}" lookup \
    --config devnet/pilot/localhost/configs/node-a.json \
    --target tcp://127.0.0.1:4111 \
    --node-id "${node_b_id}" \
    >"${node_down_lookup_log}"
  "${overlay_cli}" open-service \
    --config devnet/pilot/localhost/configs/node-a.json \
    --target tcp://127.0.0.1:4112 \
    --target-node-id "${node_b_id}" \
    --service-namespace devnet \
    --service-name terminal \
    >"${node_down_service_log}"
}

run_relay_fault_scenario() {
  stop_process "${relay_a_pid}"
  relay_a_pid=""
  if "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4198 \
    --relay-node-id "${relay_a_id}" \
    --requester-node-id "${node_a_id}" \
    >"${relay_fault_primary_log}" 2>&1; then
    echo "distributed pilot checklist: primary relay intro unexpectedly succeeded while relay was down" >&2
    exit 1
  fi
  "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4197 \
    --relay-node-id "${relay_b_id}" \
    --requester-node-id "${node_a_id}" \
    >"${relay_fault_alternate_log}"
}

run_bootstrap_seed_fault() {
  stop_process "${bootstrap_b_pid}"
  bootstrap_b_pid=""
  if [[ -n "${node_c_pid}" ]]; then
    local old_node_c_pid="${node_c_pid}"
    stop_process "${node_c_pid}"
    wait_for_lock_release "${old_node_c_pid}" "devnet/pilot/localhost/configs/node-c.json" "node-c restart path"
    node_c_pid=""
  fi
  node_c_pid="$(start_node "devnet/pilot/localhost/configs/node-c.json" "${bootstrap_fault_restart_log}")"
  wait_for_runtime "${node_c_pid}" "devnet/pilot/localhost/configs/node-c.json" "${bootstrap_fault_restart_log}" '"event":"listen","result":"ok"'
}

run_service_restart_scenario() {
  local pre_restart_status="${tmpdir}/service-pre-restart-status.json"
  wait_for_status_pattern \
    "${node_b_pid}" \
    "devnet/pilot/localhost/configs/node-b.json" \
    "${pre_restart_status}" \
    '"startup_count":' \
    'pre-restart service host status'
  local previous_startup_count
  previous_startup_count="$(extract_status_numeric_field "${pre_restart_status}" "startup_count")"

  local old_node_b_pid="${node_b_pid}"
  stop_process "${node_b_pid}"
  wait_for_lock_release "${old_node_b_pid}" "devnet/pilot/localhost/configs/node-b.json" "service host restart path"
  node_b_pid=""
  node_b_pid="$(start_node "devnet/pilot/localhost/configs/node-b.json" "${node_b_log}" --service devnet:terminal)"
  wait_for_runtime "${node_b_pid}" "devnet/pilot/localhost/configs/node-b.json" "${node_b_log}" '"event":"listen","result":"ok"'
  "${overlay_cli}" open-service \
    --config devnet/pilot/localhost/configs/node-a.json \
    --target tcp://127.0.0.1:4112 \
    --target-node-id "${node_b_id}" \
    --service-namespace devnet \
    --service-name terminal \
    >"${service_restart_log}"
  wait_for_startup_count_increment \
    "${node_b_pid}" \
    "devnet/pilot/localhost/configs/node-b.json" \
    "${service_restart_status}" \
    "$(( ${previous_startup_count:-0} + 1 ))" \
    'service host restart status'
}

run_tampered_bootstrap_check() {
  local bad_config="${tmpdir}/tampered-bootstrap-node-a.json"
  local absolute_node_key="${repo_root}/devnet/keys/node-a.key"
  sed \
    -e "s#\"node_key_path\": \"../../../keys/node-a.key\"#\"node_key_path\": \"${absolute_node_key}\"#g" \
    -e 's/"tcp_listener_addr": "127\.0\.0\.1:4111"/"tcp_listener_addr": "127.0.0.1:0"/g' \
    -e 's/sha256=[0-9a-f]\{64\}/sha256=0000000000000000000000000000000000000000000000000000000000000000/g' \
    devnet/pilot/localhost/configs/node-a.json \
    >"${bad_config}"
  "${overlay_cli}" run \
    --config "${bad_config}" \
    --max-ticks 0 \
    --status-every 1 \
    >"${tampered_bootstrap_log}" 2>&1
  grep -q '"event":"bootstrap_fetch","result":"rejected"' "${tampered_bootstrap_log}"
  grep -q '"result":"degraded"' "${tampered_bootstrap_log}"
}

start_full_topology
run_baseline_flow
run_baseline_flow_with_node_c_down
run_relay_fault_scenario
run_bootstrap_seed_fault
run_service_restart_scenario
run_tampered_bootstrap_check

cat "${baseline_publish_log}"
cat "${baseline_lookup_log}"
cat "${baseline_service_log}"
cat "${baseline_relay_a_log}"
cat "${baseline_relay_b_log}"
cat "${node_down_lookup_log}"
cat "${node_down_service_log}"
echo '{"step":"pilot_scenario","scenario":"relay-unavailable","result":"expected_degraded"}'
cat "${relay_fault_primary_log}"
cat "${relay_fault_alternate_log}"
echo '{"step":"pilot_scenario","scenario":"bootstrap-seed-unavailable","result":"ok"}'
cat "${bootstrap_fault_restart_log}"
echo '{"step":"pilot_scenario","scenario":"service-host-restart","result":"ok"}'
cat "${service_restart_log}"
echo '{"step":"pilot_scenario","scenario":"tampered-bootstrap-artifact","result":"rejected"}'
cat "${tampered_bootstrap_log}"

baseline_lookup_latency_ms="$(extract_numeric_field "${baseline_lookup_log}" "lookup_latency_ms")"
node_down_lookup_latency_ms="$(extract_numeric_field "${node_down_lookup_log}" "lookup_latency_ms")"
relay_a_bytes_last_hour="$(extract_status_numeric_field "${relay_a_status}" "total_relayed_bytes_last_hour")"
relay_b_bytes_last_hour="$(extract_status_numeric_field "${relay_b_status}" "total_relayed_bytes_last_hour")"
service_restart_startup_count="$(extract_status_numeric_field "${service_restart_status}" "startup_count")"

echo "{\"step\":\"pilot_checklist_complete\",\"topology\":\"pilot-5-node\",\"baseline\":\"ok\",\"node_down\":\"ok\",\"relay_unavailable\":\"expected_degraded\",\"bootstrap_seed_unavailable\":\"ok\",\"service_restart\":\"ok\",\"tampered_bootstrap\":\"rejected\",\"baseline_lookup_latency_ms\":${baseline_lookup_latency_ms:-0},\"node_down_lookup_latency_ms\":${node_down_lookup_latency_ms:-0},\"relay_a_bytes_last_hour\":${relay_a_bytes_last_hour:-0},\"relay_b_bytes_last_hour\":${relay_b_bytes_last_hour:-0},\"service_restart_startup_count\":${service_restart_startup_count:-0},\"relay_paths\":[\"node-a->node-relay->node-b\",\"node-a->node-relay-b->node-b\"]}"
