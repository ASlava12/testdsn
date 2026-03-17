#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
cd "${repo_root}"

evidence_dir=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --evidence-dir)
      if [[ $# -lt 2 ]]; then
        echo "distributed pilot checklist: --evidence-dir requires a path" >&2
        exit 2
      fi
      evidence_dir="$2"
      shift 2
      ;;
    -h|--help)
      echo "usage: ./devnet/run-distributed-pilot-checklist.sh [--evidence-dir <dir>]" >&2
      exit 0
      ;;
    *)
      echo "distributed pilot checklist: unknown argument '$1'" >&2
      exit 2
      ;;
  esac
done

if [[ -n "${evidence_dir}" ]]; then
  mkdir -p "${evidence_dir}"
  tmpdir="$(cd "${evidence_dir}" && pwd)"
  preserve_evidence="yes"
else
  tmpdir="$(mktemp -d)"
  preserve_evidence="no"
fi

bootstrap_a_log="${tmpdir}/pilot-bootstrap-a.log"
bootstrap_b_log="${tmpdir}/pilot-bootstrap-b.log"
bootstrap_relay_log="${tmpdir}/pilot-bootstrap-relay.log"
node_a_log="${tmpdir}/pilot-node-a.log"
node_b_log="${tmpdir}/pilot-node-b.log"
node_c_log="${tmpdir}/pilot-node-c.log"
relay_a_log="${tmpdir}/pilot-relay-a.log"
relay_b_log="${tmpdir}/pilot-relay-b.log"
relay_c_log="${tmpdir}/pilot-relay-c.log"
baseline_publish_log="${tmpdir}/baseline-publish.log"
baseline_lookup_log="${tmpdir}/baseline-lookup.log"
baseline_service_log="${tmpdir}/baseline-service.log"
baseline_relay_a_log="${tmpdir}/baseline-relay-a.log"
baseline_relay_b_log="${tmpdir}/baseline-relay-b.log"
baseline_relay_c_log="${tmpdir}/baseline-relay-c.log"
fresh_join_publish_log="${tmpdir}/fresh-join-publish.log"
fresh_join_lookup_log="${tmpdir}/fresh-join-lookup.log"
node_down_lookup_log="${tmpdir}/node-down-lookup.log"
node_down_service_log="${tmpdir}/node-down-service.log"
node_down_relay_a_log="${tmpdir}/node-down-relay-a.log"
node_down_relay_b_log="${tmpdir}/node-down-relay-b.log"
node_down_relay_c_log="${tmpdir}/node-down-relay-c.log"
relay_fault_primary_log="${tmpdir}/relay-fault-primary.log"
relay_fault_alternate_log="${tmpdir}/relay-fault-alternate.log"
relay_fault_service_log="${tmpdir}/relay-fault-service.log"
relay_recovery_primary_log="${tmpdir}/relay-recovery-primary.log"
relay_recovery_secondary_log="${tmpdir}/relay-recovery-secondary.log"
relay_recovery_tertiary_log="${tmpdir}/relay-recovery-tertiary.log"
bootstrap_fault_restart_log="${tmpdir}/bootstrap-fault-restart.log"
service_restart_log="${tmpdir}/service-restart.log"
service_restart_relay_log="${tmpdir}/service-restart-relay.log"
service_restart_status="${tmpdir}/service-restart-status.json"
trust_fallback_log="${tmpdir}/trust-fallback.log"
trust_fallback_status="${tmpdir}/trust-fallback.status.json"
tampered_bootstrap_log="${tmpdir}/tampered-bootstrap.log"
tampered_bootstrap_status="${tmpdir}/tampered-bootstrap.status.json"
integrity_fallback_log="${tmpdir}/integrity-fallback.log"
integrity_fallback_status="${tmpdir}/integrity-fallback.status.json"
stale_bootstrap_log="${tmpdir}/stale-bootstrap.log"
stale_bootstrap_status="${tmpdir}/stale-bootstrap.status.json"
empty_bootstrap_log="${tmpdir}/empty-bootstrap.log"
empty_bootstrap_status="${tmpdir}/empty-bootstrap.status.json"
relay_a_status="${tmpdir}/relay-a-status.json"
relay_b_status="${tmpdir}/relay-b-status.json"
relay_c_status="${tmpdir}/relay-c-status.json"
bootstrap_a_pid=""
bootstrap_b_pid=""
bootstrap_relay_pid=""
node_a_pid=""
node_b_pid=""
node_c_pid=""
relay_a_pid=""
relay_b_pid=""
relay_c_pid=""

cleanup() {
  status=$?
  if [[ $status -ne 0 ]]; then
    for log in \
      "${baseline_publish_log}" \
      "${baseline_lookup_log}" \
      "${baseline_service_log}" \
      "${baseline_relay_a_log}" \
      "${baseline_relay_b_log}" \
      "${baseline_relay_c_log}" \
      "${fresh_join_publish_log}" \
      "${fresh_join_lookup_log}" \
      "${node_down_lookup_log}" \
      "${node_down_service_log}" \
      "${node_down_relay_a_log}" \
      "${node_down_relay_b_log}" \
      "${node_down_relay_c_log}" \
      "${relay_fault_primary_log}" \
      "${relay_fault_alternate_log}" \
      "${relay_fault_service_log}" \
      "${relay_recovery_primary_log}" \
      "${relay_recovery_secondary_log}" \
      "${relay_recovery_tertiary_log}" \
      "${bootstrap_fault_restart_log}" \
      "${service_restart_log}" \
      "${service_restart_relay_log}" \
      "${service_restart_status}" \
      "${trust_fallback_log}" \
      "${trust_fallback_status}" \
      "${tampered_bootstrap_log}" \
      "${tampered_bootstrap_status}" \
      "${integrity_fallback_log}" \
      "${integrity_fallback_status}" \
      "${stale_bootstrap_log}" \
      "${stale_bootstrap_status}" \
      "${empty_bootstrap_log}" \
      "${empty_bootstrap_status}" \
      "${node_a_log}" \
      "${node_b_log}" \
      "${node_c_log}" \
      "${relay_a_log}" \
      "${relay_b_log}" \
      "${relay_c_log}" \
      "${bootstrap_a_log}" \
      "${bootstrap_b_log}" \
      "${bootstrap_relay_log}" \
      "${relay_a_status}" \
      "${relay_b_status}" \
      "${relay_c_status}"; do
      if [[ -f "${log}" ]]; then
        echo "--- $(basename "${log}") ---" >&2
        cat "${log}" >&2
      fi
    done
  fi
  for pid_var in node_a_pid node_b_pid node_c_pid relay_a_pid relay_b_pid relay_c_pid bootstrap_a_pid bootstrap_b_pid bootstrap_relay_pid; do
    pid="${!pid_var}"
    if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
      kill -TERM "${pid}" 2>/dev/null || true
      wait "${pid}" 2>/dev/null || true
    fi
  done
  if [[ "${preserve_evidence}" != "yes" ]]; then
    rm -rf "${tmpdir}"
  fi
  exit "${status}"
}
trap cleanup EXIT

TMPDIR=/tmp cargo build -p overlay-cli >/dev/null
overlay_cli="target/debug/overlay-cli"
mapfile -t pilot_node_a_bootstrap_sources < <(grep -o 'http://[^"]*' devnet/pilot/localhost/configs/node-a.json)
node_a_primary_bootstrap_source="${pilot_node_a_bootstrap_sources[0]}"
node_a_secondary_bootstrap_source="${pilot_node_a_bootstrap_sources[1]}"
absolute_node_a_key="${repo_root}/devnet/keys/node-a.key"

start_bootstrap_server() {
  local bind_addr="$1"
  local bootstrap_file="$2"
  local log_file="$3"
  "${overlay_cli}" bootstrap-serve \
    --bind "${bind_addr}" \
    --bootstrap-file "${bootstrap_file}" \
    --signing-key-file devnet/keys/bootstrap-signer.key \
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

wait_for_status_numeric_field_at_least() {
  local pid="$1"
  local config_path="$2"
  local status_file="$3"
  local field="$4"
  local minimum_value="$5"
  local description="$6"
  for _ in $(seq 1 240); do
    if "${overlay_cli}" status --config "${config_path}" >"${status_file}" 2>/dev/null \
      && status_matches_pid "${status_file}" "${pid}"; then
      local observed_value
      observed_value="$(extract_status_numeric_field "${status_file}" "${field}")"
      if [[ -n "${observed_value}" ]] && [[ "${observed_value}" -ge "${minimum_value}" ]]; then
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

write_temp_node_a_config() {
  local config_path="$1"
  local first_source="$2"
  local second_source="$3"
  cat >"${config_path}" <<EOF
{
  "node_key_path": "${absolute_node_a_key}",
  "bootstrap_sources": [
    "${first_source}",
    "${second_source}"
  ],
  "tcp_listener_addr": "127.0.0.1:0",
  "max_total_neighbors": 8,
  "max_presence_records": 64,
  "max_service_records": 16,
  "presence_ttl_s": 120,
  "epoch_duration_s": 60,
  "path_probe_interval_ms": 5000,
  "max_transport_buffer_bytes": 65536,
  "relay_mode": false,
  "log_level": "info"
}
EOF
}

run_bootstrap_diagnostic_config() {
  local config_path="$1"
  local log_file="$2"
  local status_file="$3"
  "${overlay_cli}" run \
    --config "${config_path}" \
    --max-ticks 0 \
    --status-every 1 \
    >"${log_file}" 2>&1
  "${overlay_cli}" status --config "${config_path}" >"${status_file}"
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
  relay_a_pid="$(start_node "devnet/pilot/localhost/configs/node-relay.json" "${relay_a_log}")"
  relay_b_pid="$(start_node "devnet/pilot/localhost/configs/node-relay-b.json" "${relay_b_log}")"
  relay_c_pid="$(start_node "devnet/pilot/localhost/configs/node-relay-c.json" "${relay_c_log}")"

  wait_for_runtime "${node_a_pid}" "devnet/pilot/localhost/configs/node-a.json" "${node_a_log}" '"event":"listen","result":"ok"'
  wait_for_runtime "${node_b_pid}" "devnet/pilot/localhost/configs/node-b.json" "${node_b_log}" '"event":"listen","result":"ok"'
  wait_for_runtime "${relay_a_pid}" "devnet/pilot/localhost/configs/node-relay.json" "${relay_a_log}" '"event":"listen","result":"ok"'
  wait_for_runtime "${relay_b_pid}" "devnet/pilot/localhost/configs/node-relay-b.json" "${relay_b_log}" '"event":"listen","result":"ok"'
  wait_for_runtime "${relay_c_pid}" "devnet/pilot/localhost/configs/node-relay-c.json" "${relay_c_log}" '"event":"listen","result":"ok"'
}

node_a_id=""
node_b_id=""
node_c_id=""
relay_a_id=""
relay_b_id=""
relay_c_id=""
relay_a_bind_total=0
relay_b_bind_total=0
relay_c_bind_total=0

run_baseline_flow() {
  node_a_id="$(extract_node_id "${node_a_pid}" "devnet/pilot/localhost/configs/node-a.json")"
  node_b_id="$(extract_node_id "${node_b_pid}" "devnet/pilot/localhost/configs/node-b.json")"
  relay_a_id="$(extract_node_id "${relay_a_pid}" "devnet/pilot/localhost/configs/node-relay.json")"
  relay_b_id="$(extract_node_id "${relay_b_pid}" "devnet/pilot/localhost/configs/node-relay-b.json")"
  relay_c_id="$(extract_node_id "${relay_c_pid}" "devnet/pilot/localhost/configs/node-relay-c.json")"

  "${overlay_cli}" publish \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4111 \
    --relay-ref "${relay_a_id}" \
    --relay-ref "${relay_b_id}" \
    --relay-ref "${relay_c_id}" \
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

  echo "{\"step\":\"relay_fallback_planned\",\"client_node\":\"node-a\",\"target_node\":\"node-b\",\"relay_node\":\"node-relay-c\",\"relay_node_id\":\"${relay_c_id}\",\"alternate_relay_node_id\":\"${relay_b_id}\"}"
  "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4196 \
    --relay-node-id "${relay_c_id}" \
    --requester-node-id "${node_a_id}" \
    >"${baseline_relay_c_log}"

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
  wait_for_status_pattern \
    "${relay_c_pid}" \
    "devnet/pilot/localhost/configs/node-relay-c.json" \
    "${relay_c_status}" \
    '"active_tunnels":1' \
    'tertiary relay tunnel bind'
  relay_a_bind_total="$(extract_status_numeric_field "${relay_a_status}" "relay_bind_total")"
  relay_b_bind_total="$(extract_status_numeric_field "${relay_b_status}" "relay_bind_total")"
  relay_c_bind_total="$(extract_status_numeric_field "${relay_c_status}" "relay_bind_total")"
}

run_fresh_join_scenario() {
  node_c_pid="$(start_node "devnet/pilot/localhost/configs/node-c.json" "${node_c_log}")"
  wait_for_runtime "${node_c_pid}" "devnet/pilot/localhost/configs/node-c.json" "${node_c_log}" '"event":"listen","result":"ok"'
  node_c_id="$(extract_node_id "${node_c_pid}" "devnet/pilot/localhost/configs/node-c.json")"

  "${overlay_cli}" publish \
    --config devnet/pilot/localhost/configs/node-c.json \
    --target tcp://127.0.0.1:4111 \
    --relay-ref "${relay_a_id}" \
    --relay-ref "${relay_b_id}" \
    --relay-ref "${relay_c_id}" \
    >"${fresh_join_publish_log}"

  "${overlay_cli}" lookup \
    --config devnet/pilot/localhost/configs/node-a.json \
    --target tcp://127.0.0.1:4111 \
    --node-id "${node_c_id}" \
    >"${fresh_join_lookup_log}"
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
  "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4198 \
    --relay-node-id "${relay_a_id}" \
    --requester-node-id "${node_a_id}" \
    >"${node_down_relay_a_log}"
  "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4197 \
    --relay-node-id "${relay_b_id}" \
    --requester-node-id "${node_a_id}" \
    >"${node_down_relay_b_log}"
  "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4196 \
    --relay-node-id "${relay_c_id}" \
    --requester-node-id "${node_a_id}" \
    >"${node_down_relay_c_log}"
  wait_for_status_numeric_field_at_least \
    "${relay_a_pid}" \
    "devnet/pilot/localhost/configs/node-relay.json" \
    "${relay_a_status}" \
    "relay_bind_total" \
    "$(( ${relay_a_bind_total:-0} + 1 ))" \
    'node-c-down relay bind increment on primary relay'
  relay_a_bind_total="$(extract_status_numeric_field "${relay_a_status}" "relay_bind_total")"
  wait_for_status_numeric_field_at_least \
    "${relay_b_pid}" \
    "devnet/pilot/localhost/configs/node-relay-b.json" \
    "${relay_b_status}" \
    "relay_bind_total" \
    "$(( ${relay_b_bind_total:-0} + 1 ))" \
    'node-c-down relay bind increment on alternate relay'
  relay_b_bind_total="$(extract_status_numeric_field "${relay_b_status}" "relay_bind_total")"
  wait_for_status_numeric_field_at_least \
    "${relay_c_pid}" \
    "devnet/pilot/localhost/configs/node-relay-c.json" \
    "${relay_c_status}" \
    "relay_bind_total" \
    "$(( ${relay_c_bind_total:-0} + 1 ))" \
    'node-c-down relay bind increment on tertiary relay'
  relay_c_bind_total="$(extract_status_numeric_field "${relay_c_status}" "relay_bind_total")"
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
  "${overlay_cli}" open-service \
    --config devnet/pilot/localhost/configs/node-a.json \
    --target tcp://127.0.0.1:4112 \
    --target-node-id "${node_b_id}" \
    --service-namespace devnet \
    --service-name terminal \
    >"${relay_fault_service_log}"
  wait_for_status_numeric_field_at_least \
    "${relay_b_pid}" \
    "devnet/pilot/localhost/configs/node-relay-b.json" \
    "${relay_b_status}" \
    "relay_bind_total" \
    "$(( ${relay_b_bind_total:-0} + 1 ))" \
    'relay-unavailable alternate relay bind increment'
  relay_b_bind_total="$(extract_status_numeric_field "${relay_b_status}" "relay_bind_total")"
}

run_repeated_relay_failure_scenario() {
  stop_process "${relay_b_pid}"
  relay_b_pid=""
  if "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4198 \
    --relay-node-id "${relay_a_id}" \
    --requester-node-id "${node_a_id}" \
    >"${relay_recovery_primary_log}" 2>&1; then
    echo "distributed pilot checklist: primary relay intro unexpectedly succeeded during repeated-failure recovery" >&2
    exit 1
  fi
  if "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4197 \
    --relay-node-id "${relay_b_id}" \
    --requester-node-id "${node_a_id}" \
    >"${relay_recovery_secondary_log}" 2>&1; then
    echo "distributed pilot checklist: secondary relay intro unexpectedly succeeded during repeated-failure recovery" >&2
    exit 1
  fi
  "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4196 \
    --relay-node-id "${relay_c_id}" \
    --requester-node-id "${node_a_id}" \
    >"${relay_recovery_tertiary_log}"
  wait_for_status_numeric_field_at_least \
    "${relay_c_pid}" \
    "devnet/pilot/localhost/configs/node-relay-c.json" \
    "${relay_c_status}" \
    "relay_bind_total" \
    "$(( ${relay_c_bind_total:-0} + 1 ))" \
    'repeated-failure tertiary relay bind increment'
  relay_c_bind_total="$(extract_status_numeric_field "${relay_c_status}" "relay_bind_total")"
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
  node_b_pid="$(start_node "devnet/pilot/localhost/configs/node-b.json" "${node_b_log}")"
  wait_for_runtime "${node_b_pid}" "devnet/pilot/localhost/configs/node-b.json" "${node_b_log}" '"event":"listen","result":"ok"'
  "${overlay_cli}" open-service \
    --config devnet/pilot/localhost/configs/node-a.json \
    --target tcp://127.0.0.1:4112 \
    --target-node-id "${node_b_id}" \
    --service-namespace devnet \
    --service-name terminal \
    >"${service_restart_log}"
  "${overlay_cli}" relay-intro \
    --config devnet/pilot/localhost/configs/node-b.json \
    --target tcp://127.0.0.1:4197 \
    --relay-node-id "${relay_b_id}" \
    --requester-node-id "${node_a_id}" \
    >"${service_restart_relay_log}"
  wait_for_status_numeric_field_at_least \
    "${relay_b_pid}" \
    "devnet/pilot/localhost/configs/node-relay-b.json" \
    "${relay_b_status}" \
    "relay_bind_total" \
    "$(( ${relay_b_bind_total:-0} + 1 ))" \
    'service-host-restart alternate relay bind increment'
  relay_b_bind_total="$(extract_status_numeric_field "${relay_b_status}" "relay_bind_total")"
  wait_for_startup_count_increment \
    "${node_b_pid}" \
    "devnet/pilot/localhost/configs/node-b.json" \
    "${service_restart_status}" \
    "$(( ${previous_startup_count:-0} + 1 ))" \
    'service host restart status'
  grep -q '"restored_service_intents":1' "${service_restart_status}"
  grep -q '"recoverable_service_intents":1' "${service_restart_status}"
  grep -q '"failed_service_intents":0' "${service_restart_status}"
}

run_integrity_fallback_check() {
  local config_path="${tmpdir}/integrity-fallback-node-a.json"
  local bad_primary_source
  bad_primary_source="$(printf '%s\n' "${node_a_primary_bootstrap_source}" | sed 's/sha256=[0-9a-f]\{64\}/sha256=0000000000000000000000000000000000000000000000000000000000000000/')"
  write_temp_node_a_config "${config_path}" "${bad_primary_source}" "${node_a_secondary_bootstrap_source}"
  run_bootstrap_diagnostic_config "${config_path}" "${integrity_fallback_log}" "${integrity_fallback_status}"
  grep -q '"state":"running"' "${integrity_fallback_log}"
  grep -q '"integrity_mismatch_sources":1' "${integrity_fallback_status}"
  grep -q '"accepted_sources":1' "${integrity_fallback_status}"
  grep -q '"result":"integrity_mismatch"' "${integrity_fallback_status}"
  grep -q '"result":"accepted"' "${integrity_fallback_status}"
}

run_trust_fallback_check() {
  local config_path="${tmpdir}/trust-fallback-node-a.json"
  local bad_primary_source
  bad_primary_source="$(printf '%s\n' "${node_a_primary_bootstrap_source}" | sed 's/ed25519=[0-9a-f]\{64\}/ed25519=0000000000000000000000000000000000000000000000000000000000000000/')"
  write_temp_node_a_config "${config_path}" "${bad_primary_source}" "${node_a_secondary_bootstrap_source}"
  run_bootstrap_diagnostic_config "${config_path}" "${trust_fallback_log}" "${trust_fallback_status}"
  grep -q '"state":"running"' "${trust_fallback_log}"
  grep -q '"trust_verification_failed_sources":1' "${trust_fallback_status}"
  grep -q '"accepted_sources":1' "${trust_fallback_status}"
  grep -q '"result":"trust_verification_failed"' "${trust_fallback_status}"
  grep -q '"result":"accepted"' "${trust_fallback_status}"
}

run_stale_bootstrap_check() {
  local config_path="${tmpdir}/stale-bootstrap-node-a.json"
  local stale_bootstrap_file="${tmpdir}/stale-bootstrap.json"
  cat >"${stale_bootstrap_file}" <<'EOF'
{
  "version": 1,
  "generated_at_unix_s": 1,
  "expires_at_unix_s": 2,
  "network_params": {
    "network_id": "overlay-devnet"
  },
  "epoch_duration_s": 60,
  "presence_ttl_s": 120,
  "max_frame_body_len": 65519,
  "handshake_version": 1,
  "peers": [
    {
      "node_id": [30, 237, 41, 177, 101, 79, 188, 169, 70, 23, 0, 77, 121, 105, 223, 196, 101, 43, 31, 48, 167, 168, 183, 113, 195, 72, 0, 21, 84, 131, 56, 11],
      "transport_classes": ["quic", "tcp"],
      "capabilities": ["service-host"],
      "dial_hints": ["tcp://127.0.0.1:4112"],
      "observed_role": "standard"
    }
  ],
  "bridge_hints": []
}
EOF
  write_temp_node_a_config "${config_path}" "file:${stale_bootstrap_file}" "${node_a_secondary_bootstrap_source}"
  run_bootstrap_diagnostic_config "${config_path}" "${stale_bootstrap_log}" "${stale_bootstrap_status}"
  grep -q '"state":"running"' "${stale_bootstrap_log}"
  grep -q '"stale_sources":1' "${stale_bootstrap_status}"
  grep -q '"accepted_sources":1' "${stale_bootstrap_status}"
  grep -q '"result":"stale"' "${stale_bootstrap_status}"
  grep -q '"result":"accepted"' "${stale_bootstrap_status}"
}

run_empty_bootstrap_check() {
  local config_path="${tmpdir}/empty-bootstrap-node-a.json"
  local empty_bootstrap_file="${tmpdir}/empty-bootstrap.json"
  cat >"${empty_bootstrap_file}" <<'EOF'
{
  "version": 1,
  "generated_at_unix_s": 1900000000,
  "expires_at_unix_s": 2000000000,
  "network_params": {
    "network_id": "overlay-devnet"
  },
  "epoch_duration_s": 60,
  "presence_ttl_s": 120,
  "max_frame_body_len": 65519,
  "handshake_version": 1,
  "peers": [],
  "bridge_hints": []
}
EOF
  write_temp_node_a_config "${config_path}" "file:${empty_bootstrap_file}" "${node_a_secondary_bootstrap_source}"
  run_bootstrap_diagnostic_config "${config_path}" "${empty_bootstrap_log}" "${empty_bootstrap_status}"
  grep -q '"state":"running"' "${empty_bootstrap_log}"
  grep -q '"empty_peer_set_sources":1' "${empty_bootstrap_status}"
  grep -q '"accepted_sources":1' "${empty_bootstrap_status}"
  grep -q '"result":"empty_peer_set"' "${empty_bootstrap_status}"
  grep -q '"result":"accepted"' "${empty_bootstrap_status}"
}

run_tampered_bootstrap_check() {
  local bad_config="${tmpdir}/tampered-bootstrap-node-a.json"
  local bad_primary_source
  local bad_secondary_source
  bad_primary_source="$(printf '%s\n' "${node_a_primary_bootstrap_source}" | sed 's/sha256=[0-9a-f]\{64\}/sha256=0000000000000000000000000000000000000000000000000000000000000000/')"
  bad_secondary_source="$(printf '%s\n' "${node_a_secondary_bootstrap_source}" | sed 's/sha256=[0-9a-f]\{64\}/sha256=0000000000000000000000000000000000000000000000000000000000000000/')"
  write_temp_node_a_config "${bad_config}" "${bad_primary_source}" "${bad_secondary_source}"
  run_bootstrap_diagnostic_config "${bad_config}" "${tampered_bootstrap_log}" "${tampered_bootstrap_status}"
  grep -q '"event":"bootstrap_fetch","result":"integrity_mismatch"' "${tampered_bootstrap_log}"
  grep -q '"state":"degraded"' "${tampered_bootstrap_log}"
  grep -q '"integrity_mismatch_sources":2' "${tampered_bootstrap_status}"
}

start_full_topology
run_baseline_flow
run_fresh_join_scenario
run_baseline_flow_with_node_c_down
run_relay_fault_scenario
run_service_restart_scenario
run_repeated_relay_failure_scenario
run_bootstrap_seed_fault
run_integrity_fallback_check
run_trust_fallback_check
run_stale_bootstrap_check
run_empty_bootstrap_check
run_tampered_bootstrap_check

cat "${baseline_publish_log}"
cat "${baseline_lookup_log}"
cat "${baseline_service_log}"
cat "${baseline_relay_a_log}"
cat "${baseline_relay_b_log}"
cat "${baseline_relay_c_log}"
echo '{"step":"pilot_scenario","scenario":"service-publish","result":"ok"}'
echo '{"step":"pilot_scenario","scenario":"service-discover-and-open","result":"ok"}'
echo '{"step":"pilot_scenario","scenario":"direct-path-loss-relay-fallback","result":"ok"}'
echo '{"step":"pilot_scenario","scenario":"three-relay-candidate-set","result":"ok"}'
echo '{"step":"pilot_scenario","scenario":"fresh-node-join","result":"ok"}'
cat "${fresh_join_publish_log}"
cat "${fresh_join_lookup_log}"
cat "${node_down_lookup_log}"
cat "${node_down_service_log}"
cat "${node_down_relay_a_log}"
cat "${node_down_relay_b_log}"
cat "${node_down_relay_c_log}"
echo '{"step":"pilot_scenario","scenario":"node-c-down","result":"ok"}'
echo '{"step":"pilot_scenario","scenario":"relay-unavailable","result":"expected_degraded"}'
cat "${relay_fault_primary_log}"
cat "${relay_fault_alternate_log}"
echo '{"step":"pilot_scenario","scenario":"relay-unavailable-service-open","result":"ok"}'
cat "${relay_fault_service_log}"
echo '{"step":"pilot_scenario","scenario":"repeated-relay-bind-failure-recovery","result":"ok"}'
cat "${relay_recovery_primary_log}"
cat "${relay_recovery_secondary_log}"
cat "${relay_recovery_tertiary_log}"
echo '{"step":"pilot_scenario","scenario":"bootstrap-seed-unavailable","result":"ok"}'
cat "${bootstrap_fault_restart_log}"
echo '{"step":"pilot_scenario","scenario":"service-host-restart","result":"ok"}'
cat "${service_restart_log}"
cat "${service_restart_relay_log}"
echo '{"step":"pilot_scenario","scenario":"integrity-mismatch-fallback","result":"ok"}'
cat "${integrity_fallback_log}"
echo '{"step":"pilot_scenario","scenario":"trust-verification-fallback","result":"ok"}'
cat "${trust_fallback_log}"
echo '{"step":"pilot_scenario","scenario":"stale-bootstrap-fallback","result":"ok"}'
cat "${stale_bootstrap_log}"
echo '{"step":"pilot_scenario","scenario":"empty-bootstrap-fallback","result":"ok"}'
cat "${empty_bootstrap_log}"
echo '{"step":"pilot_scenario","scenario":"tampered-bootstrap-artifact","result":"rejected"}'
cat "${tampered_bootstrap_log}"

baseline_lookup_latency_ms="$(extract_numeric_field "${baseline_lookup_log}" "lookup_latency_ms")"
fresh_join_lookup_latency_ms="$(extract_numeric_field "${fresh_join_lookup_log}" "lookup_latency_ms")"
node_down_lookup_latency_ms="$(extract_numeric_field "${node_down_lookup_log}" "lookup_latency_ms")"
relay_a_bytes_last_hour="$(extract_status_numeric_field "${relay_a_status}" "total_relayed_bytes_last_hour")"
relay_b_bytes_last_hour="$(extract_status_numeric_field "${relay_b_status}" "total_relayed_bytes_last_hour")"
relay_c_bytes_last_hour="$(extract_status_numeric_field "${relay_c_status}" "total_relayed_bytes_last_hour")"
service_restart_startup_count="$(extract_status_numeric_field "${service_restart_status}" "startup_count")"

echo "{\"step\":\"pilot_checklist_complete\",\"topology\":\"pilot-6-node-3-relay\",\"relay_candidate_count\":3,\"fresh_node_join\":\"ok\",\"service_publish\":\"ok\",\"service_open\":\"ok\",\"direct_path_loss_relay_fallback\":\"ok\",\"three_relay_candidate_set\":\"ok\",\"baseline\":\"ok\",\"node_down\":\"ok\",\"relay_unavailable\":\"expected_degraded\",\"relay_unavailable_service_open\":\"ok\",\"repeated_relay_bind_failure_recovery\":\"ok\",\"bootstrap_seed_unavailable\":\"ok\",\"integrity_mismatch_fallback\":\"ok\",\"trust_verification_fallback\":\"ok\",\"stale_bootstrap_fallback\":\"ok\",\"empty_bootstrap_fallback\":\"ok\",\"service_restart\":\"ok\",\"tampered_bootstrap\":\"rejected\",\"baseline_lookup_latency_ms\":${baseline_lookup_latency_ms:-0},\"fresh_join_lookup_latency_ms\":${fresh_join_lookup_latency_ms:-0},\"node_down_lookup_latency_ms\":${node_down_lookup_latency_ms:-0},\"relay_a_bytes_last_hour\":${relay_a_bytes_last_hour:-0},\"relay_b_bytes_last_hour\":${relay_b_bytes_last_hour:-0},\"relay_c_bytes_last_hour\":${relay_c_bytes_last_hour:-0},\"relay_a_bind_total\":${relay_a_bind_total:-0},\"relay_b_bind_total\":${relay_b_bind_total:-0},\"relay_c_bind_total\":${relay_c_bind_total:-0},\"service_restart_startup_count\":${service_restart_startup_count:-0},\"relay_paths\":[\"node-a->node-relay->node-b\",\"node-a->node-relay-b->node-b\",\"node-a->node-relay-c->node-b\"]}"
if [[ "${preserve_evidence}" == "yes" ]]; then
  echo "{\"step\":\"pilot_evidence_bundle\",\"path\":\"${tmpdir}\"}"
fi
