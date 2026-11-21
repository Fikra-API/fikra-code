#!/usr/bin/env bash
# Safe local build for small machines.
#  - Hard RAM cap via cgroup: if the build exceeds it, only the build is killed.
#  - Watchdog: if the whole machine drops below FIKRA_MIN_AVAIL_MB free, the
#    build is stopped so the desktop never hangs.
#  - Low CPU/IO priority, 1 job (FIKRA_BUILD_JOBS to change), no debug info, incremental.
#
# Usage: scripts/fikra/dev-build.sh [check|build|<any cargo subcommand> ...]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT/codex-rs"

MEM="${FIKRA_BUILD_MEM:-2500M}"
JOBS="${FIKRA_BUILD_JOBS:-1}"
MIN_AVAIL_MB="${FIKRA_MIN_AVAIL_MB:-600}"
UNIT="fikra-build-$$"

export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_INCREMENTAL=1
export CARGO_BUILD_JOBS="$JOBS"

cmd="${1:-build}"
shift || true
case "$cmd" in
  check) args=(check -p codex-cli --bin fikra "$@") ;;
  # `cargo rustc` passes the flag to the final binary only, so cached
  # dependencies stay valid. Single-threaded lld keeps the link within RAM.
  build) args=(rustc -p codex-cli --bin fikra "$@" -- -C link-arg=-Wl,--threads=1) ;;
  *) args=("$cmd" "$@") ;;
esac

watchdog() {
  while sleep 3; do
    avail=$(awk '/MemAvailable/ {print int($2/1024)}' /proc/meminfo)
    if (( avail < MIN_AVAIL_MB )); then
      echo "[dev-build] machine has only ${avail}MB free; stopping build to protect the system" >&2
      systemctl --user kill --signal=SIGKILL "${UNIT}.scope" 2>/dev/null || true
      return
    fi
  done
}
watchdog &
WD=$!
trap 'kill $WD 2>/dev/null || true' EXIT

systemd-run --user --scope -q --unit="$UNIT" \
  -p MemoryMax="$MEM" -p MemorySwapMax=1G -p CPUWeight=20 -p IOWeight=20 \
  -- nice -n 19 ionice -c3 cargo "${args[@]}"
