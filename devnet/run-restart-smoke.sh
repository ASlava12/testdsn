#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
config_path="${repo_root}/docs/config-examples/service-host-node.json"

cd "${repo_root}"
TMPDIR=/tmp cargo run -p overlay-cli -- run --config "${config_path}" --max-ticks 0 --status-every 1
TMPDIR=/tmp cargo run -p overlay-cli -- run --config "${config_path}" --max-ticks 0 --status-every 1
