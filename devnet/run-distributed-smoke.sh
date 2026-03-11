#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

tmpdir="$(mktemp -d)"
bootstrap_a_log="$tmpdir/bootstrap-a.log"
bootstrap_b_log="$tmpdir/bootstrap-b.log"
bootstrap_relay_log="$tmpdir/bootstrap-relay.log"
server_log="$tmpdir/node-b.log"
client_log="$tmpdir/node-a.log"
bootstrap_a_pid=""
bootstrap_b_pid=""
bootstrap_relay_pid=""
server_pid=""
client_pid=""

cleanup() {
  status=$?
  if [[ $status -ne 0 ]]; then
    if [[ -f "$server_log" ]]; then
      echo "--- node-b.log ---" >&2
      cat "$server_log" >&2
    fi
    if [[ -f "$client_log" ]]; then
      echo "--- node-a.log ---" >&2
      cat "$client_log" >&2
    fi
    for log in "$bootstrap_a_log" "$bootstrap_b_log" "$bootstrap_relay_log"; do
      if [[ -f "$log" ]]; then
        echo "--- $(basename "$log") ---" >&2
        cat "$log" >&2
      fi
    done
  fi
  if [[ -n "${client_pid}" ]] && kill -0 "${client_pid}" 2>/dev/null; then
    kill "${client_pid}" 2>/dev/null || true
    wait "${client_pid}" 2>/dev/null || true
  fi
  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" 2>/dev/null; then
    kill "${server_pid}" 2>/dev/null || true
    wait "${server_pid}" 2>/dev/null || true
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
      echo "distributed smoke: bootstrap server $bind_addr exited before startup" >&2
      exit 1
    fi
    sleep 0.05
  done
  cat "$log_file" >&2
  echo "distributed smoke: bootstrap server $bind_addr did not report readiness" >&2
  exit 1
}

bootstrap_a_pid="$(start_bootstrap_server "127.0.0.1:4201" "devnet/hosts/examples/bootstrap/node-foundation.json" "$bootstrap_a_log")"
bootstrap_b_pid="$(start_bootstrap_server "127.0.0.1:4202" "devnet/hosts/examples/bootstrap/node-a-seed.json" "$bootstrap_b_log")"
bootstrap_relay_pid="$(start_bootstrap_server "127.0.0.1:4203" "devnet/hosts/examples/bootstrap/node-ab-seed.json" "$bootstrap_relay_log")"

"$overlay_cli" run \
  --config devnet/hosts/localhost/configs/node-b.json \
  --tick-ms 25 \
  --max-ticks 200 \
  >"$server_log" 2>&1 &
server_pid="$!"

for _ in $(seq 1 200); do
  if grep -q '"component":"transport","event":"listen","result":"ok"' "$server_log"; then
    break
  fi
  if ! kill -0 "$server_pid" 2>/dev/null; then
    cat "$server_log"
    echo "distributed smoke: node-b exited before listener startup" >&2
    exit 1
  fi
  sleep 0.05
done

if ! grep -q '"component":"transport","event":"listen","result":"ok"' "$server_log"; then
  cat "$server_log"
  echo "distributed smoke: listener startup log not observed" >&2
  exit 1
fi

"$overlay_cli" run \
  --config devnet/hosts/localhost/configs/node-a.json \
  --tick-ms 25 \
  --max-ticks 200 \
  --dial tcp://127.0.0.1:4102 \
  >"$client_log" 2>&1 &
client_pid="$!"

wait "$client_pid"
client_pid=""
wait "$server_pid"
server_pid=""

grep -q '"component":"bootstrap","event":"bootstrap_fetch","result":"accepted"' "$server_log"
grep -q '"component":"bootstrap","event":"bootstrap_fetch","result":"accepted"' "$client_log"
grep -q '"component":"transport","event":"dial","result":"started"' "$client_log"
grep -q '"component":"session","event":"open_succeeded","result":"ok"' "$client_log"
grep -q '"component":"transport","event":"accept","result":"accepted"' "$server_log"
grep -q '"component":"session","event":"open_succeeded","result":"ok"' "$server_log"

echo '{"step":"distributed_smoke_complete","client":"node-a","server":"node-b","transport":"tcp","bootstrap":"http"}'
