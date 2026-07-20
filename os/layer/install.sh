#!/usr/bin/env bash
# Lisa Layer installer (Track L, ADR-0003): the §5 stack on stock Arch or
# Omarchy — no custom image required.
#
# M0 STUB: validates the host and shows the plan; it installs nothing yet.
# It becomes real when os/packages produces the first signed pacman repo
# (M0→M1 backlog, PLAN Appendix D). The uninstaller mirrors every step.
set -euo pipefail

say() { printf '%s\n' "$*"; }

if [ ! -f /etc/arch-release ]; then
    say "error: the Lisa Layer targets Arch Linux (including Omarchy)." >&2
    exit 1
fi

say "Lisa Layer installer (M0 stub — dry run only)"
say ""
say "When the package repo ships, this script will:"
say "  1. Add the [lisa] pacman repo (signed) to /etc/pacman.conf"
say "  2. Install: lisa-inferenced lisa-modeld lisa-cli (M1 set)"
say "  3. Enable user services: lisa-inferenced.service (no-network sandbox)"
say "  4. Configure Snapper pre-update snapshots ('/' only, quotas off)"
say "     with Limine boot-menu restore — see os/layer/snapper/"
say ""
say "Nothing was changed. Track progress: docs/PLAN.md §10 (M0/M1)."
