#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"

tmpdir="$(mktemp -d)"
package_dir="${tmpdir}/packages"
extract_dir="${tmpdir}/extract"
install_prefix="${tmpdir}/install-root"

mkdir -p "${package_dir}" "${extract_dir}"
cd "${repo_root}"

"${script_dir}/package-release.sh" --output-dir "${package_dir}" > "${tmpdir}/package.log"

tarball_path="$(find "${package_dir}" -maxdepth 1 -type f -name '*.tar.gz' | head -n 1)"
checksum_path="${tarball_path}.sha256"

if [[ -z "${tarball_path}" || ! -f "${checksum_path}" ]]; then
  echo "packaging check: missing release tarball or checksum" >&2
  exit 1
fi

(cd "${package_dir}" && sha256sum -c "$(basename "${checksum_path}")") >/dev/null
tar -C "${extract_dir}" -xzf "${tarball_path}"

package_root="$(find "${extract_dir}" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
if [[ -z "${package_root}" ]]; then
  echo "packaging check: extracted package root not found" >&2
  exit 1
fi

test -f "${package_root}/install.sh"
test -x "${package_root}/bin/overlay-cli"
test -f "${package_root}/docs/PRODUCTION_CHECKLIST.md"
test -f "${package_root}/docs/KNOWN_LIMITATIONS.md"
test -d "${package_root}/examples/config-examples"
test -d "${package_root}/examples/pilot/configs"

if find "${package_root}" -type f \( -name '*.key' -o -name 'bootstrap-signer.key' \) | grep -q .; then
  echo "packaging check: release package unexpectedly contains private key material" >&2
  exit 1
fi

"${package_root}/install.sh" --prefix "${install_prefix}" > "${tmpdir}/install.log"

test -x "${install_prefix}/bin/overlay-cli"
test -f "${install_prefix}/share/overlay/docs/PRODUCTION_CHECKLIST.md"
test -f "${install_prefix}/share/overlay/docs/KNOWN_LIMITATIONS.md"
test -f "${install_prefix}/share/overlay/examples/pilot/configs/node-relay-c.json"

expected_stage="$(tr -d '\r\n' < "${repo_root}/REPOSITORY_STAGE")"
installed_stage="$("${install_prefix}/bin/overlay-cli")"
echo "${installed_stage}" | grep -q "${expected_stage}"

"${install_prefix}/bin/overlay-cli" config-template --profile relay-capable --output "${tmpdir}/relay-capable.json"
grep -q '"relay_mode": true' "${tmpdir}/relay-capable.json"

grep -q '"step":"release_package_complete"' "${tmpdir}/package.log"
grep -q '"step":"release_install_complete"' "${tmpdir}/install.log"

echo "{\"step\":\"packaging_check_complete\",\"package\":\"${tarball_path}\",\"installed_prefix\":\"${install_prefix}\",\"stage\":\"${expected_stage}\",\"private_keys_in_package\":false}"
