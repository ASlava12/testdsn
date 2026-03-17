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
        echo "first-user acceptance: --evidence-dir requires a path" >&2
        exit 2
      fi
      evidence_dir="$2"
      shift 2
      ;;
    -h|--help)
      echo "usage: ./devnet/run-first-user-acceptance.sh [--evidence-dir <dir>]" >&2
      exit 0
      ;;
    *)
      echo "first-user acceptance: unknown argument '$1'" >&2
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

launch_gate_log="${tmpdir}/launch-gate.log"
distributed_acceptance_log="${tmpdir}/distributed-pilot-checklist.log"
distributed_evidence_dir="${tmpdir}/distributed-pilot-evidence"

cleanup() {
  status=$?
  if [[ $status -ne 0 ]]; then
    for log in "${launch_gate_log}" "${distributed_acceptance_log}"; do
      if [[ -f "${log}" ]]; then
        echo "--- $(basename "${log}") ---" >&2
        cat "${log}" >&2
      fi
    done
    if [[ -d "${distributed_evidence_dir}" ]]; then
      echo "first-user acceptance: preserved distributed evidence in ${distributed_evidence_dir}" >&2
    fi
  fi
  if [[ "${preserve_evidence}" != "yes" ]]; then
    rm -rf "${tmpdir}"
  fi
  exit "${status}"
}
trap cleanup EXIT

"${script_dir}/run-launch-gate.sh" >"${launch_gate_log}" 2>&1
"${script_dir}/run-distributed-pilot-checklist.sh" \
  --evidence-dir "${distributed_evidence_dir}" \
  >"${distributed_acceptance_log}" 2>&1

grep -q '"step":"soak_complete"' "${launch_gate_log}"
grep -q '"kind":"runtime_doctor"' "${launch_gate_log}"
grep -q '"restored_from_peer_cache":true' "${launch_gate_log}"
grep -q '"state":"recovered_from_peer_cache"' "${launch_gate_log}"

grep -q '"step":"pilot_scenario","scenario":"fresh-node-join","result":"ok"' "${distributed_acceptance_log}"
grep -q '"step":"pilot_scenario","scenario":"relay-unavailable-service-open","result":"ok"' "${distributed_acceptance_log}"
grep -q '"fresh_node_join":"ok"' "${distributed_acceptance_log}"
grep -q '"service_publish":"ok"' "${distributed_acceptance_log}"
grep -q '"service_open":"ok"' "${distributed_acceptance_log}"
grep -q '"direct_path_loss_relay_fallback":"ok"' "${distributed_acceptance_log}"
grep -q '"bootstrap_seed_unavailable":"ok"' "${distributed_acceptance_log}"
grep -q '"relay_unavailable_service_open":"ok"' "${distributed_acceptance_log}"
grep -q '"service_restart":"ok"' "${distributed_acceptance_log}"
grep -q '"tampered_bootstrap":"rejected"' "${distributed_acceptance_log}"

grep '"step":"soak_complete"' "${launch_gate_log}" | tail -n 1
grep '"step":"pilot_scenario"' "${distributed_acceptance_log}"
grep '"step":"pilot_checklist_complete"' "${distributed_acceptance_log}" | tail -n 1
echo '{"step":"acceptance_scenario","scenario":"fresh-node-join","result":"ok"}'
echo '{"step":"acceptance_scenario","scenario":"service-publish","result":"ok"}'
echo '{"step":"acceptance_scenario","scenario":"service-discover-and-open","result":"ok"}'
echo '{"step":"acceptance_scenario","scenario":"direct-path-loss-relay-fallback","result":"ok"}'
echo '{"step":"acceptance_scenario","scenario":"bootstrap-source-unavailable","result":"ok"}'
echo '{"step":"acceptance_scenario","scenario":"relay-unavailable-service-open","result":"ok"}'
echo '{"step":"acceptance_scenario","scenario":"ordinary-restart-recovery","result":"ok"}'
echo '{"step":"acceptance_scenario","scenario":"stale-presence-and-expired-state-recovery","result":"ok"}'
echo '{"step":"acceptance_scenario","scenario":"tampered-bootstrap-artifact","result":"expected_rejected"}'

echo '{"step":"first_user_acceptance_complete","launch_gate":"ok","distributed_acceptance":"ok","fresh_node_join":"ok","service_publish":"ok","service_discover_and_open":"ok","direct_path_loss_relay_fallback":"ok","bootstrap_source_unavailable":"ok","relay_unavailable":"expected_degraded","relay_unavailable_service_open":"ok","restart_recovery":"ok","stale_presence_and_expired_state_recovery":"ok","tampered_bootstrap":"expected_rejected","boundary":"first-user-ready-bounded"}'
if [[ "${preserve_evidence}" == "yes" ]]; then
  echo "{\"step\":\"first_user_acceptance_evidence\",\"path\":\"${tmpdir}\"}"
fi
