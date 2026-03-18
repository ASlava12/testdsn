#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"

soak_seconds=1800
status_interval_seconds=300

while [[ $# -gt 0 ]]; do
  case "$1" in
    --soak-seconds)
      if [[ $# -lt 2 ]]; then
        echo "run-soak: --soak-seconds requires an integer value" >&2
        exit 2
      fi
      soak_seconds="$2"
      shift 2
      ;;
    --status-interval-seconds)
      if [[ $# -lt 2 ]]; then
        echo "run-soak: --status-interval-seconds requires an integer value" >&2
        exit 2
      fi
      status_interval_seconds="$2"
      shift 2
      ;;
    -h|--help)
      echo "usage: ./devnet/run-soak.sh [--soak-seconds <seconds>] [--status-interval-seconds <seconds>]" >&2
      exit 0
      ;;
    *)
      echo "run-soak: unknown argument '$1'" >&2
      exit 2
      ;;
  esac
done

cd "${repo_root}"
TMPDIR=/tmp cargo run -p overlay-cli -- smoke --devnet-dir "${script_dir}" --soak-seconds "${soak_seconds}" --status-interval-seconds "${status_interval_seconds}"
