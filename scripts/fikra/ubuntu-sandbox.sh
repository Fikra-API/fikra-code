#!/bin/sh
# Let Fikra Code's sandbox (bubblewrap) run on Ubuntu 23.10+ / 24.04+.
#
# Ubuntu restricts unprivileged user namespaces with AppArmor
# (kernel.apparmor_restrict_unprivileged_userns=1). bubblewrap then cannot set
# up its sandbox ("setting up uid map: Permission denied" or
# "loopback: Failed RTM_NEWADDR"), every command fails inside the sandbox, and
# Fikra Code has to ask permission to run each one outside it. This installs an
# AppArmor profile that allows user namespaces for bubblewrap only, which is
# Ubuntu's documented way to permit a specific program.
#
# Usage: curl -fsSL https://fikraapi.co.ke/code/ubuntu-sandbox.sh | sudo sh
# Undo:  sudo rm /etc/apparmor.d/fikra-bwrap && sudo systemctl reload apparmor
set -eu

if [ "$(id -u)" -ne 0 ]; then
  echo "Run with sudo: curl -fsSL https://fikraapi.co.ke/code/ubuntu-sandbox.sh | sudo sh" >&2
  exit 1
fi
if [ "$(sysctl -n kernel.apparmor_restrict_unprivileged_userns 2>/dev/null || echo 0)" != "1" ]; then
  echo "This system does not restrict user namespaces. Nothing to do."
  exit 0
fi
if ! command -v apparmor_parser >/dev/null 2>&1; then
  echo "apparmor_parser not found; cannot install the profile." >&2
  exit 1
fi

PROFILE=/etc/apparmor.d/fikra-bwrap
SYSTEM_BWRAP="$(command -v bwrap 2>/dev/null || true)"
{
  echo "# Installed by Fikra Code: allow bubblewrap to create user namespaces."
  echo "abi <abi/4.0>,"
  echo "include <tunables/global>"
  echo
  if [ -n "$SYSTEM_BWRAP" ]; then
    echo "profile fikra-bwrap $SYSTEM_BWRAP flags=(unconfined) {"
    echo "  userns,"
    echo "  include if exists <local/fikra-bwrap>"
    echo "}"
    echo
  fi
  echo "profile fikra-bwrap-bundled @{HOME}/.fikra/packages/*/codex-resources/bwrap flags=(unconfined) {"
  echo "  userns,"
  echo "  include if exists <local/fikra-bwrap-bundled>"
  echo "}"
} > "$PROFILE"
apparmor_parser -r "$PROFILE"
echo "Done. Fikra Code's sandbox can now run commands without asking each time."
