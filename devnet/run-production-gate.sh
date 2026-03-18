#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"

evidence_dir=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --evidence-dir)
      if [[ $# -lt 2 ]]; then
        echo "production gate: --evidence-dir requires a path" >&2
        exit 2
      fi
      evidence_dir="$2"
      shift 2
      ;;
    -h|--help)
      echo "usage: ./devnet/run-production-gate.sh [--evidence-dir <dir>]" >&2
      exit 0
      ;;
    *)
      echo "production gate: unknown argument '$1'" >&2
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

cd "${repo_root}"

acceptance_log="${tmpdir}/first-user-acceptance.log"
production_soak_log="${tmpdir}/production-soak.log"
packaging_log="${tmpdir}/packaging-check.log"
acceptance_evidence_dir="${tmpdir}/first-user-acceptance-evidence"

cleanup() {
  status=$?
  if [[ $status -ne 0 ]]; then
    for log in "${acceptance_log}" "${production_soak_log}" "${packaging_log}"; do
      if [[ -f "${log}" ]]; then
        echo "--- $(basename "${log}") ---" >&2
        cat "${log}" >&2
      fi
    done
  fi
  if [[ "${preserve_evidence}" != "yes" ]]; then
    rm -rf "${tmpdir}"
  fi
  exit "${status}"
}
trap cleanup EXIT

"${script_dir}/run-first-user-acceptance.sh" \
  --evidence-dir "${acceptance_evidence_dir}" \
  > "${acceptance_log}" 2>&1
"${script_dir}/run-production-soak.sh" > "${production_soak_log}" 2>&1
"${script_dir}/run-packaging-check.sh" > "${packaging_log}" 2>&1

grep -q '"step":"first_user_acceptance_complete"' "${acceptance_log}"
grep -q '"step":"soak_complete"' "${production_soak_log}"
grep -q '"soak_seconds":3600' "${production_soak_log}"
grep -q '"step":"packaging_check_complete"' "${packaging_log}"

grep '"step":"first_user_acceptance_complete"' "${acceptance_log}" | tail -n 1
grep '"step":"soak_complete"' "${production_soak_log}" | tail -n 1
grep '"step":"packaging_check_complete"' "${packaging_log}" | tail -n 1

echo '{"step":"production_gate_component","component":"first-user-acceptance","result":"ok"}'
echo '{"step":"production_gate_component","component":"production-soak","result":"ok"}'
echo '{"step":"production_gate_component","component":"packaging-check","result":"ok"}'
echo '{"step":"production_gate_complete","first_user_acceptance":"ok","production_soak":"ok","packaging_check":"ok","boundary":"bounded-production-release"}'
if [[ "${preserve_evidence}" == "yes" ]]; then
  echo "{\"step\":\"production_gate_evidence\",\"path\":\"${tmpdir}\"}"
fi
