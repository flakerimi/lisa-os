#!/usr/bin/env bash
# Track L end-to-end, run INSIDE an Arch container with systemd as PID1
# (M0 acceptance, PLAN §10): build the [lisa] repo from git HEAD →
# install.sh → service up → lisa ask smoke → uninstall.sh → vanilla.
#
# Launch (host with podman; repo mounted read-only at /src):
#   podman run -d --name lisa-e2e --systemd=always -v "$PWD":/src:ro \
#     docker.io/library/archlinux:latest /usr/lib/systemd/systemd
#   podman exec lisa-e2e bash /src/tests/e2e/layer-test.sh
#   podman rm -f lisa-e2e
set -euo pipefail

say() { printf '\n== %s\n' "$*"; }

say "provision build deps"
pacman -Syu --noconfirm --needed base-devel rust git curl

say "clean checkout of HEAD (tests exactly what is committed)"
git clone --quiet /src /build
useradd -m builder 2>/dev/null || true
chown -R builder /build

say "build the [lisa] repo"
sudo -u builder bash -c 'cd /build && bash os/repo-tools/build-packages.sh /build/repo-out'

say "install the layer"
LISA_REPO_URL=file:///build/repo-out bash /build/os/layer/install.sh

say "verify: service active, endpoint answers, CLI round-trips"
systemctl is-active lisa-inferenced.service
curl -sf 127.0.0.1:7777/health | grep -q '"status":"ok"'
lisa ask --no-stream "layer-e2e-canary" | grep -q "layer-e2e-canary"
pacman -Qq lisa-inferenced lisa-modeld lisa-cli >/dev/null

say "uninstall the layer"
LISA_ASSUME_YES=1 bash /build/os/layer/uninstall.sh

say "verify: vanilla system"
if pacman -Qq lisa-inferenced lisa-modeld lisa-cli 2>/dev/null | grep -q .; then
    echo "FAIL: packages still installed" >&2; exit 1
fi
if grep -q '^\[lisa\]' /etc/pacman.conf; then
    echo "FAIL: [lisa] stanza still in pacman.conf" >&2; exit 1
fi
if [ -f /usr/lib/systemd/system/lisa-inferenced.service ]; then
    echo "FAIL: unit file still present" >&2; exit 1
fi
if systemctl is-active --quiet lisa-inferenced.service 2>/dev/null; then
    echo "FAIL: service still running" >&2; exit 1
fi

say "LAYER E2E: PASS"
