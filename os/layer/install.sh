#!/usr/bin/env bash
# Lisa Layer installer (Track L, ADR-0003): the Lisa stack on stock Arch
# or Omarchy — no custom image required. Root required (pacman).
#
# Repo source: set LISA_REPO_URL (e.g. file:///path/to/repo built by
# os/repo-tools/build-packages.sh). The hosted, signed repo lands in M1;
# until then SigLevel is Optional for explicitly local repos.
set -euo pipefail

say() { printf '%s\n' "$*"; }
die() { say "error: $*" >&2; exit 1; }

[ -f /etc/arch-release ] || die "the Lisa Layer targets Arch Linux (including Omarchy)."
[ "$(id -u)" -eq 0 ] || die "run as root (pacman needs it): sudo $0"
[ -n "${LISA_REPO_URL:-}" ] || die "LISA_REPO_URL is not set.
The hosted repo is not live yet (M1). Build locally instead:
  os/repo-tools/build-packages.sh /srv/lisa-repo
  sudo LISA_REPO_URL=file:///srv/lisa-repo $0"

if grep -q '^\[lisa\]' /etc/pacman.conf; then
    say ">> [lisa] repo already configured, leaving pacman.conf untouched"
else
    say ">> adding [lisa] repo to /etc/pacman.conf"
    cat >>/etc/pacman.conf <<EOF

# BEGIN lisa layer (added by os/layer/install.sh; removed by uninstall.sh)
[lisa]
SigLevel = Optional TrustAll
Server = $LISA_REPO_URL
# END lisa layer
EOF
fi

say ">> installing lisa-inferenced lisa-modeld lisa-cli (full -Syu: partial upgrades break Arch)"
pacman -Syu --noconfirm lisa-inferenced lisa-modeld lisa-cli

if [ -d /run/systemd/system ]; then
    say ">> enabling lisa-inferenced.service"
    systemctl daemon-reload
    systemctl enable --now lisa-inferenced.service
    say ">> waiting for the inference endpoint"
    # LISA_HEALTH_TIMEOUT: seconds to wait (default 10; CI containers are slow).
    for _ in $(seq 1 $((5 * ${LISA_HEALTH_TIMEOUT:-10}))); do
        curl -sf 127.0.0.1:7777/health >/dev/null 2>&1 && break
        sleep 0.2
    done
    curl -sf 127.0.0.1:7777/health >/dev/null || die "lisa-inferenced did not come up; see: journalctl -u lisa-inferenced"
else
    say ">> no running systemd detected; skipping service enablement"
fi

say ""
say "Lisa Layer installed. Try:  lisa ask \"write a haiku about entropy\""
say "Uninstall cleanly with:     os/layer/uninstall.sh"
