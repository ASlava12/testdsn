#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

tmpdir="$(mktemp -d)"
server_log="$tmpdir/node-b.log"
client_log="$tmpdir/node-a.log"
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
  fi
  if [[ -n "${client_pid}" ]] && kill -0 "${client_pid}" 2>/dev/null; then
    kill "${client_pid}" 2>/dev/null || true
    wait "${client_pid}" 2>/dev/null || true
  fi
  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" 2>/dev/null; then
    kill "${server_pid}" 2>/dev/null || true
    wait "${server_pid}" 2>/dev/null || true
  fi
  rm -rf "$tmpdir"
  exit $status
}
trap cleanup EXIT

TMPDIR=/tmp cargo build -p overlay-cli >/dev/null

target/debug/overlay-cli run \
  --config devnet/configs/node-b.json \
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

target/debug/overlay-cli run \
  --config devnet/configs/node-a.json \
  --tick-ms 25 \
  --max-ticks 200 \
  --dial tcp://127.0.0.1:4102 \
  >"$client_log" 2>&1 &
client_pid="$!"

wait "$client_pid"
client_pid=""
wait "$server_pid"
server_pid=""

grep -q '"component":"transport","event":"dial","result":"started"' "$client_log"
grep -q '"component":"session","event":"open_succeeded","result":"ok"' "$client_log"
grep -q '"component":"transport","event":"accept","result":"accepted"' "$server_log"
grep -q '"component":"session","event":"open_succeeded","result":"ok"' "$server_log"

echo '{"step":"distributed_smoke_complete","client":"node-a","server":"node-b","transport":"tcp"}'
