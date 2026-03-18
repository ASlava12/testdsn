#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
package_root="${script_dir}"

prefix="${HOME}/.local"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix)
      if [[ $# -lt 2 ]]; then
        echo "install-overlay: --prefix requires a path" >&2
        exit 2
      fi
      prefix="$2"
      shift 2
      ;;
    -h|--help)
      echo "usage: ./install.sh [--prefix <dir>]" >&2
      exit 0
      ;;
    *)
      echo "install-overlay: unknown argument '$1'" >&2
      exit 2
      ;;
  esac
done

bin_dir="${prefix}/bin"
share_dir="${prefix}/share/overlay"

if [[ ! -x "${package_root}/bin/overlay-cli" ]]; then
  echo "install-overlay: packaged bundle layout not found next to install.sh" >&2
  exit 1
fi

mkdir -p "${bin_dir}" "${share_dir}/docs" "${share_dir}/examples"
install -m 0755 "${package_root}/bin/overlay-cli" "${bin_dir}/overlay-cli"
cp -R "${package_root}/docs/." "${share_dir}/docs"
cp -R "${package_root}/examples/." "${share_dir}/examples"
install -m 0644 "${package_root}/README.md" "${share_dir}/README.md"
install -m 0644 "${package_root}/LICENSE" "${share_dir}/LICENSE"
install -m 0644 "${package_root}/REPOSITORY_STAGE" "${share_dir}/REPOSITORY_STAGE"
install -m 0644 "${package_root}/BUILD_INFO.json" "${share_dir}/BUILD_INFO.json"
install -m 0644 "${package_root}/MANIFEST.txt" "${share_dir}/MANIFEST.txt"
install -m 0755 "${package_root}/install.sh" "${share_dir}/install.sh"

stage_marker="$(tr -d '\r\n' < "${package_root}/REPOSITORY_STAGE")"
echo "{\"step\":\"release_install_complete\",\"prefix\":\"${prefix}\",\"stage\":\"${stage_marker}\"}"
