#!/usr/bin/env bash
# End-to-end checks for a built `fikra` binary. Runs the same way locally and in CI.
#
# Usage: FIKRA_API_KEY=... scripts/fikra/smoke.sh path/to/fikra
#
# Checks:
#   a) --version / --help contain no OpenAI/Codex branding
#   b) login accepts a valid key, rejects an invalid one, logout works
#   c) the agent fixes a real bug using the default model (no --model flag)
#   d) during c), no connection is made to any OpenAI/ChatGPT host
set -euo pipefail

BIN="$(cd "$(dirname "${1:?usage: smoke.sh path/to/fikra}")" && pwd)/$(basename "$1")"
: "${FIKRA_API_KEY:?FIKRA_API_KEY must be set}"
HERE="$(cd "$(dirname "$0")" && pwd)"
WORK="$(mktemp -d)"
trap 'kill "${PROXY_PID:-}" 2>/dev/null || true; rm -rf "$WORK"' EXIT

export FIKRA_HOME="$WORK/home"
mkdir -p "$FIKRA_HOME"
# Login must work from the saved key alone, so keep the env var out of the
# binary's environment and pass it only to `login` via stdin.
KEY="$FIKRA_API_KEY"
unset FIKRA_API_KEY OPENAI_API_KEY CODEX_API_KEY

fail() { echo "FAIL: $*" >&2; exit 1; }

echo "=== a) branding ==="
"$BIN" --version | tee "$WORK/version.txt"
"$BIN" --help > "$WORK/help.txt"
if grep -Eqi 'codex|openai|chatgpt' "$WORK/version.txt"; then fail "--version mentions Codex/OpenAI"; fi
if grep -Eqi 'openai|chatgpt' "$WORK/help.txt"; then fail "--help mentions OpenAI/ChatGPT"; fi

echo "=== b) login / rejection / logout ==="
printf '%s' "invalid-key-123" | "$BIN" login && fail "invalid key was accepted"
printf '%s' "$KEY" | "$BIN" login || fail "valid key was rejected"
"$BIN" login status || fail "login status failed after login"
"$BIN" logout || fail "logout failed"
"$BIN" login status && fail "still logged in after logout"
printf '%s' "$KEY" | "$BIN" login >/dev/null || fail "re-login failed"

echo "=== c+d) bug-fix task through a host-logging proxy ==="
TASK="$WORK/task"
mkdir -p "$TASK"
printf 'def add(a, b):\n    return a - b\n' > "$TASK/calc.py"
cat > "$TASK/test_calc.py" <<'PY'
import unittest
from calc import add
class TestCalc(unittest.TestCase):
    def test_add(self):
        self.assertEqual(add(2, 3), 5)
if __name__ == "__main__":
    unittest.main()
PY
HOSTS="$WORK/hosts.log"
: > "$HOSTS"
PORT="${FIKRA_SMOKE_PROXY_PORT:-8899}"
python3 "$HERE/connect_proxy.py" "$HOSTS" "$PORT" &
PROXY_PID=$!
for _ in $(seq 50); do grep -q . "$HOSTS.ready" 2>/dev/null && break; sleep 0.1; done
export HTTPS_PROXY="http://127.0.0.1:$PORT" HTTP_PROXY="http://127.0.0.1:$PORT"
export https_proxy="$HTTPS_PROXY" http_proxy="$HTTP_PROXY"

(cd "$TASK" && timeout 600 "$BIN" exec -C "$TASK" \
  --dangerously-bypass-approvals-and-sandbox --skip-git-repo-check \
  "Run the tests, find the bug, fix it, and run the tests again." 2>&1) | tee "$WORK/exec.log"
unset HTTPS_PROXY HTTP_PROXY https_proxy http_proxy

grep -q 'return a + b' "$TASK/calc.py" || fail "calc.py was not fixed"
(cd "$TASK" && python3 -m unittest -q) || fail "tests still fail"

if grep -Eq '\b(Codex|OpenAI)\b' "$WORK/exec.log"; then
  grep -En '\b(Codex|OpenAI)\b' "$WORK/exec.log" | head -5
  fail "exec output mentions Codex/OpenAI"
fi

echo "=== hosts contacted ==="
sort -u "$HOSTS"
if grep -Eqi '(^|\.)(openai\.com|chatgpt\.com|oaistatic\.com|oaiusercontent\.com)$' "$HOSTS"; then
  fail "connected to an OpenAI/ChatGPT host"
fi
grep -q 'fikraapi.co.ke' "$HOSTS" || fail "never reached fikraapi.co.ke (proxy not used?)"
OTHER="$(sort -u "$HOSTS" | grep -vx 'fikraapi.co.ke' || true)"
[[ -z "$OTHER" ]] || fail "contacted hosts other than fikraapi.co.ke: $OTHER"

echo "=== e) interactive app screen ==="
if python3 -c "import pyte" 2>/dev/null; then
  TUI="$WORK/tui"; mkdir -p "$TUI" && (cd "$TUI" && git init -q .)
  # A stale OpenAI model in config must be replaced by the Fikra default.
  printf 'model = "gpt-5.6-terra"\n' >> "$FIKRA_HOME/config.toml"
  SCREEN="$(cd "$TUI" && python3 "$HERE/tui_snap.py" 24 '1\r' "$BIN" 2>&1 || true)"
  echo "$SCREEN" | tail -15
  echo "$SCREEN" | grep -Eq '\b(Codex|OpenAI)\b' && fail "interactive app mentions Codex/OpenAI"
  echo "$SCREEN" | grep -qP '[\x{2800}-\x{28FF}]' && fail "interactive app draws the upstream logo"
  echo "$SCREEN" | grep -q 'gpt-' && fail "interactive app shows a GPT model"
  echo "$SCREEN" | grep -q 'deepseek-v4.1-flash' || fail "interactive app is not on the Fikra default model"
else
  echo "pyte not installed; skipping the interactive screen check"
fi

echo "ALL SMOKE CHECKS PASSED"
