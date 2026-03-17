#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
tmpdir="$(mktemp -d)"
config_path="${tmpdir}/user-node.json"
keys_dir="${tmpdir}/keys"
bootstrap_dir="${tmpdir}/bootstrap"
status_output="${tmpdir}/runtime-status.json"
doctor_output="${tmpdir}/doctor.json"
runtime_log="${tmpdir}/doctor-runtime.log"
node_pid=""

cleanup() {
  status=$?
  if [[ $status -ne 0 ]]; then
    for file in "${runtime_log}" "${status_output}" "${doctor_output}"; do
      if [[ -f "${file}" ]]; then
        echo "--- $(basename "${file}") ---" >&2
        cat "${file}" >&2
      fi
    done
  fi
  if [[ -n "${node_pid}" ]] && kill -0 "${node_pid}" 2>/dev/null; then
    kill -TERM "${node_pid}" 2>/dev/null || true
    wait "${node_pid}" 2>/dev/null || true
  fi
  rm -rf "${tmpdir}"
  exit $status
}
trap cleanup EXIT

cd "${repo_root}"
TMPDIR=/tmp cargo build -p overlay-cli >/dev/null
overlay_cli="target/debug/overlay-cli"

mkdir -p "${keys_dir}" "${bootstrap_dir}"
cp "${repo_root}/devnet/keys/node-a.key" "${keys_dir}/node.key"
cp "${repo_root}/devnet/bootstrap/node-foundation.json" "${bootstrap_dir}/node-foundation.json"
"${overlay_cli}" config-template --profile user-node --output "${config_path}" >/dev/null
# Doctor only needs a healthy runtime snapshot; avoid a listener dependency here.
sed -i '/"tcp_listener_addr":/d' "${config_path}"

"${overlay_cli}" run \
  --config "${config_path}" \
  --tick-ms 25 \
  --status-every 1 \
  >"${runtime_log}" 2>&1 &
node_pid="$!"

for _ in $(seq 1 200); do
  if "${overlay_cli}" status --config "${config_path}" >"${status_output}" 2>/dev/null; then
    if grep -q '"state":"running"' "${status_output}"; then
      break
    fi
  fi
  if ! kill -0 "${node_pid}" 2>/dev/null; then
    cat "${runtime_log}" >&2
    echo "doctor smoke: runtime exited before status became available" >&2
    exit 1
  fi
  sleep 0.05
done

"${overlay_cli}" doctor --config "${config_path}" >"${doctor_output}"
grep -q '"result":"ok"' "${doctor_output}"
grep -q '"name":"bootstrap","result":"ok"' "${doctor_output}"
grep -q '"name":"runtime_state","result":"ok"' "${doctor_output}"

kill -TERM "${node_pid}"
wait "${node_pid}"
node_pid=""

cat "${doctor_output}"
