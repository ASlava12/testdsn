#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"

cd "${repo_root}"
TMPDIR=/tmp cargo run -p overlay-cli -- smoke --devnet-dir "${script_dir}" --soak-seconds 1800 --status-interval-seconds 300
