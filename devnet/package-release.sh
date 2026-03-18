#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"

output_dir="${repo_root}/target/release-packages"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output-dir)
      if [[ $# -lt 2 ]]; then
        echo "package-release: --output-dir requires a path" >&2
        exit 2
      fi
      output_dir="$2"
      shift 2
      ;;
    -h|--help)
      echo "usage: ./devnet/package-release.sh [--output-dir <dir>]" >&2
      exit 0
      ;;
    *)
      echo "package-release: unknown argument '$1'" >&2
      exit 2
      ;;
  esac
done

mkdir -p "${output_dir}"
cd "${repo_root}"

version="$(sed -n 's/^version = "\(.*\)"/\1/p' crates/overlay-cli/Cargo.toml | head -n 1)"
if [[ -z "${version}" ]]; then
  echo "package-release: could not determine overlay-cli version" >&2
  exit 1
fi
stage_marker="$(tr -d '\r\n' < REPOSITORY_STAGE)"
target_triple="$(rustc -vV | sed -n 's/^host: //p')"
package_name="overlay-v${version}-${target_triple}"
tarball_path="${output_dir}/${package_name}.tar.gz"
checksum_path="${tarball_path}.sha256"
staging_dir="$(mktemp -d "${output_dir}/package-staging.XXXXXX")"
bundle_root="${staging_dir}/${package_name}"

TMPDIR=/tmp cargo build --locked --release -p overlay-cli

mkdir -p "${bundle_root}/bin" "${bundle_root}/docs" "${bundle_root}/examples"
install -m 0755 "target/release/overlay-cli" "${bundle_root}/bin/overlay-cli"
install -m 0755 "packaging/install-overlay.sh" "${bundle_root}/install.sh"
install -m 0644 "README.md" "${bundle_root}/README.md"
install -m 0644 "LICENSE" "${bundle_root}/LICENSE"
install -m 0644 "REPOSITORY_STAGE" "${bundle_root}/REPOSITORY_STAGE"

cp "docs/PRODUCTION_CHECKLIST.md" "${bundle_root}/docs/PRODUCTION_CHECKLIST.md"
cp "docs/PRODUCTION_RELEASE_TEMPLATE.md" "${bundle_root}/docs/PRODUCTION_RELEASE_TEMPLATE.md"
cp "docs/KNOWN_LIMITATIONS.md" "${bundle_root}/docs/KNOWN_LIMITATIONS.md"
cp "docs/FIRST_USER_ACCEPTANCE.md" "${bundle_root}/docs/FIRST_USER_ACCEPTANCE.md"
cp "docs/PILOT_RUNBOOK.md" "${bundle_root}/docs/PILOT_RUNBOOK.md"
cp "docs/PILOT_REPORT_TEMPLATE.md" "${bundle_root}/docs/PILOT_REPORT_TEMPLATE.md"
cp "docs/CONFIG_EXAMPLES.md" "${bundle_root}/docs/CONFIG_EXAMPLES.md"
cp "docs/DEVNET.md" "${bundle_root}/docs/DEVNET.md"
cp "docs/RUNBOOK.md" "${bundle_root}/docs/RUNBOOK.md"
cp "docs/TROUBLESHOOTING.md" "${bundle_root}/docs/TROUBLESHOOTING.md"
cp -R "docs/config-examples" "${bundle_root}/examples/config-examples"
cp -R "devnet/hosts/examples" "${bundle_root}/examples/hosts"
cp -R "devnet/pilot/examples" "${bundle_root}/examples/pilot"

printf '{\n  "package_name": "%s",\n  "version": "%s",\n  "stage": "%s",\n  "target_triple": "%s"\n}\n' \
  "${package_name}" \
  "${version}" \
  "${stage_marker}" \
  "${target_triple}" \
  > "${bundle_root}/BUILD_INFO.json"

find "${bundle_root}" -type f | sort | sed "s#${bundle_root}/##" > "${bundle_root}/MANIFEST.txt"
tar -C "${staging_dir}" -czf "${tarball_path}" "${package_name}"
(cd "${output_dir}" && sha256sum "$(basename "${tarball_path}")" > "$(basename "${checksum_path}")")

echo "{\"step\":\"release_package_complete\",\"package\":\"${tarball_path}\",\"checksum\":\"${checksum_path}\",\"stage\":\"${stage_marker}\",\"target\":\"${target_triple}\"}"
