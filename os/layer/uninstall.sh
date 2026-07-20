#!/usr/bin/env bash
# Lisa Layer uninstaller (Track L, ADR-0003). M0 STUB — mirrors
# install.sh: shows the plan, changes nothing yet. The real uninstaller
# must leave a vanilla system: packages removed, repo stanza removed,
# services disabled, user data (models, context) removed only after an
# explicit per-item prompt.
set -euo pipefail

say() { printf '%s\n' "$*"; }

say "Lisa Layer uninstaller (M0 stub — dry run only)"
say ""
say "When real, this script will (each step confirmed):"
say "  1. Disable and remove lisa user services"
say "  2. Remove lisa packages and the [lisa] pacman repo stanza"
say "  3. Ask — separately — before touching model store or context data"
say ""
say "Nothing was changed."
