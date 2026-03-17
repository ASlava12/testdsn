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
        echo "multihost smoke: --evidence-dir requires a path" >&2
        exit 2
      fi
      evidence_dir="$2"
      shift 2
      ;;
    -h|--help)
      echo "usage: ./devnet/run-multihost-smoke.sh [--evidence-dir <dir>]" >&2
      exit 0
      ;;
    *)
      echo "multihost smoke: unknown argument '$1'" >&2
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

bootstrap_a_log="${tmpdir}/bootstrap-a.log"
bootstrap_b_log="${tmpdir}/bootstrap-b.log"
bootstrap_relay_log="${tmpdir}/bootstrap-relay.log"
node_a_log="${tmpdir}/node-a.log"
node_b_log="${tmpdir}/node-b.log"
node_c_log="${tmpdir}/node-c.log"
relay_log="${tmpdir}/node-relay.log"
publish_log="${tmpdir}/publish.log"
lookup_log="${tmpdir}/lookup.log"
service_log="${tmpdir}/service.log"
relay_intro_log="${tmpdir}/relay-intro.log"
relay_status_log="${tmpdir}/relay-status.json"
bootstrap_a_pid=""
bootstrap_b_pid=""
bootstrap_relay_pid=""
node_a_pid=""
node_b_pid=""
node_c_pid=""
relay_pid=""

cleanup() {
  status=$?
  if [[ $status -ne 0 ]]; then
    for log in \
      "${publish_log}" \
      "${lookup_log}" \
      "${service_log}" \
      "${relay_intro_log}" \
      "${node_a_log}" \
      "${node_b_log}" \
      "${node_c_log}" \
      "${relay_log}" \
      "${bootstrap_a_log}" \
      "${bootstrap_b_log}" \
      "${bootstrap_relay_log}" \
      "${relay_status_log}"; do
      if [[ -f "${log}" ]]; then
        echo "--- $(basename "${log}") ---" >&2
        cat "${log}" >&2
      fi
    done
  fi
  for pid_var in node_a_pid node_b_pid node_c_pid relay_pid bootstrap_a_pid bootstrap_b_pid bootstrap_relay_pid; do
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
      echo "multihost smoke: bootstrap server ${bind_addr} exited before startup" >&2
      exit 1
    fi
    sleep 0.05
  done
  cat "${log_file}" >&2
  echo "multihost smoke: bootstrap server ${bind_addr} did not report readiness" >&2
  exit 1
}

start_node() {
  local name="$1"
  local config_path="$2"
  local log_file="$3"
  shift 3
  "${overlay_cli}" run \
    --config "${config_path}" \
    --tick-ms 25 \
    --max-ticks 600 \
    "$@" \
    >"${log_file}" 2>&1 &
  printf '%s\n' "$!"
}

status_probe_file() {
  local config_path="$1"
  printf '%s/%s.status.json\n' "${tmpdir}" "$(basename "${config_path}" .json)"
}

status_matches_pid() {
  local status_file="$1"
  local pid="$2"
  grep -q "\"pid\":${pid}" "${status_file}"
}

wait_for_runtime() {
  local pid="$1"
  local config_path="$2"
  local log_file="$3"
  local listener_log="${4:-}"
  local status_file
  status_file="$(status_probe_file "${config_path}")"
  for _ in $(seq 1 240); do
    if [[ -n "${listener_log}" ]] && ! grep -q "${listener_log}" "${log_file}"; then
      :
    elif "${overlay_cli}" status --config "${config_path}" >"${status_file}" 2>/dev/null \
      && status_matches_pid "${status_file}" "${pid}"; then
      return 0
    fi
    if ! kill -0 "${pid}" 2>/dev/null; then
      cat "${log_file}" >&2
      echo "multihost smoke: runtime ${config_path} exited before status became available" >&2
      exit 1
    fi
    sleep 0.05
  done
  cat "${log_file}" >&2
  echo "multihost smoke: runtime ${config_path} did not become ready" >&2
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
      echo "multihost smoke: runtime ${config_path} exited before ${description}" >&2
      exit 1
    fi
    sleep 0.05
  done
  echo "multihost smoke: runtime ${config_path} did not reach ${description}" >&2
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

bootstrap_a_pid="$(start_bootstrap_server "127.0.0.1:4201" "devnet/hosts/examples/bootstrap/node-foundation.json" "${bootstrap_a_log}")"
bootstrap_b_pid="$(start_bootstrap_server "127.0.0.1:4202" "devnet/hosts/examples/bootstrap/node-a-seed.json" "${bootstrap_b_log}")"
bootstrap_relay_pid="$(start_bootstrap_server "127.0.0.1:4203" "devnet/hosts/examples/bootstrap/node-ab-seed.json" "${bootstrap_relay_log}")"

node_a_pid="$(start_node "node-a" "devnet/hosts/localhost/configs/node-a.json" "${node_a_log}")"
node_b_pid="$(start_node "node-b" "devnet/hosts/localhost/configs/node-b.json" "${node_b_log}" --service devnet:terminal)"
node_c_pid="$(start_node "node-c" "devnet/hosts/localhost/configs/node-c.json" "${node_c_log}")"
relay_pid="$(start_node "node-relay" "devnet/hosts/localhost/configs/node-relay.json" "${relay_log}")"

wait_for_runtime "${node_a_pid}" "devnet/hosts/localhost/configs/node-a.json" "${node_a_log}" '"event":"listen","result":"ok"'
wait_for_runtime "${node_b_pid}" "devnet/hosts/localhost/configs/node-b.json" "${node_b_log}" '"event":"listen","result":"ok"'
wait_for_runtime "${node_c_pid}" "devnet/hosts/localhost/configs/node-c.json" "${node_c_log}" '"event":"listen","result":"ok"'
wait_for_runtime "${relay_pid}" "devnet/hosts/localhost/configs/node-relay.json" "${relay_log}" '"event":"listen","result":"ok"'

node_a_id="$(extract_node_id "${node_a_pid}" "devnet/hosts/localhost/configs/node-a.json")"
node_b_id="$(extract_node_id "${node_b_pid}" "devnet/hosts/localhost/configs/node-b.json")"
relay_node_id="$(extract_node_id "${relay_pid}" "devnet/hosts/localhost/configs/node-relay.json")"

echo "{\"step\":\"startup\",\"node\":\"node-a\",\"node_id\":\"${node_a_id}\"}"
echo "{\"step\":\"startup\",\"node\":\"node-b\",\"node_id\":\"${node_b_id}\"}"
echo "{\"step\":\"startup\",\"node\":\"node-c\"}"
echo "{\"step\":\"startup\",\"node\":\"node-relay\",\"node_id\":\"${relay_node_id}\"}"

"${overlay_cli}" publish \
  --config devnet/hosts/localhost/configs/node-b.json \
  --target tcp://127.0.0.1:4101 \
  --relay-ref "${relay_node_id}" \
  --capability service-host \
  >"${publish_log}"

"${overlay_cli}" lookup \
  --config devnet/hosts/localhost/configs/node-a.json \
  --target tcp://127.0.0.1:4101 \
  --node-id "${node_b_id}" \
  >"${lookup_log}"

"${overlay_cli}" open-service \
  --config devnet/hosts/localhost/configs/node-a.json \
  --target tcp://127.0.0.1:4102 \
  --target-node-id "${node_b_id}" \
  --service-namespace devnet \
  --service-name terminal \
  >"${service_log}"

echo "{\"step\":\"relay_fallback_planned\",\"client_node\":\"node-a\",\"target_node\":\"node-b\",\"relay_node\":\"node-relay\",\"requester_node_id\":\"${node_a_id}\"}"

"${overlay_cli}" relay-intro \
  --config devnet/hosts/localhost/configs/node-b.json \
  --target tcp://127.0.0.1:4199 \
  --relay-node-id "${relay_node_id}" \
  --requester-node-id "${node_a_id}" \
  >"${relay_intro_log}"

wait_for_status_pattern \
  "${relay_pid}" \
  "devnet/hosts/localhost/configs/node-relay.json" \
  "${relay_status_log}" \
  '"active_tunnels":1' \
  'relay tunnel bind'

cat "${publish_log}"
cat "${lookup_log}"
cat "${service_log}"
cat "${relay_intro_log}"

echo "{\"step\":\"smoke_complete\",\"result\":\"ok\",\"node_a_id\":\"${node_a_id}\",\"node_b_id\":\"${node_b_id}\",\"relay_node_id\":\"${relay_node_id}\"}"
if [[ "${preserve_evidence}" == "yes" ]]; then
  echo "{\"step\":\"smoke_evidence_bundle\",\"path\":\"${tmpdir}\"}"
fi
