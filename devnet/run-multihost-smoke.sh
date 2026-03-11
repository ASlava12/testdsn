#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

tmpdir="$(mktemp -d)"
bootstrap_a_log="$tmpdir/bootstrap-a.log"
bootstrap_b_log="$tmpdir/bootstrap-b.log"
bootstrap_relay_log="$tmpdir/bootstrap-relay.log"
smoke_log="$tmpdir/multihost-smoke.log"
bootstrap_a_pid=""
bootstrap_b_pid=""
bootstrap_relay_pid=""

cleanup() {
  status=$?
  if [[ $status -ne 0 ]]; then
    if [[ -f "$smoke_log" ]]; then
      echo "--- multihost-smoke.log ---" >&2
      cat "$smoke_log" >&2
    fi
    for log in "$bootstrap_a_log" "$bootstrap_b_log" "$bootstrap_relay_log"; do
      if [[ -f "$log" ]]; then
        echo "--- $(basename "$log") ---" >&2
        cat "$log" >&2
      fi
    done
  fi
  for pid_var in bootstrap_a_pid bootstrap_b_pid bootstrap_relay_pid; do
    pid="${!pid_var}"
    if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
      kill "${pid}" 2>/dev/null || true
      wait "${pid}" 2>/dev/null || true
    fi
  done
  rm -rf "$tmpdir"
  exit $status
}
trap cleanup EXIT

TMPDIR=/tmp cargo build -p overlay-cli >/dev/null
overlay_cli="target/debug/overlay-cli"

start_bootstrap_server() {
  local bind_addr="$1"
  local bootstrap_file="$2"
  local log_file="$3"
  "$overlay_cli" bootstrap-serve \
    --bind "$bind_addr" \
    --bootstrap-file "$bootstrap_file" \
    >"$log_file" 2>&1 &
  local pid="$!"
  for _ in $(seq 1 200); do
    if grep -q '"step":"bootstrap_server_listen"' "$log_file"; then
      printf '%s\n' "$pid"
      return 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      cat "$log_file" >&2
      echo "multihost smoke: bootstrap server $bind_addr exited before startup" >&2
      exit 1
    fi
    sleep 0.05
  done
  cat "$log_file" >&2
  echo "multihost smoke: bootstrap server $bind_addr did not report readiness" >&2
  exit 1
}

bootstrap_a_pid="$(start_bootstrap_server "127.0.0.1:4201" "devnet/bootstrap/node-foundation.json" "$bootstrap_a_log")"
bootstrap_b_pid="$(start_bootstrap_server "127.0.0.1:4202" "devnet/bootstrap/node-a-seed.json" "$bootstrap_b_log")"
bootstrap_relay_pid="$(start_bootstrap_server "127.0.0.1:4203" "devnet/bootstrap/node-ab-seed.json" "$bootstrap_relay_log")"

"$overlay_cli" smoke --devnet-dir devnet/hosts/localhost >"$smoke_log" 2>&1

grep -q '"step":"startup"' "$smoke_log"
grep -q '"step":"session_established"' "$smoke_log"
grep -q '"step":"publish_presence"' "$smoke_log"
grep -q '"step":"lookup_node"' "$smoke_log"
grep -q '"step":"open_service"' "$smoke_log"
grep -q '"step":"relay_fallback_planned"' "$smoke_log"
grep -q '"step":"relay_fallback_bound"' "$smoke_log"
grep -q '"step":"smoke_complete"' "$smoke_log"

cat "$smoke_log"
