#!/usr/bin/env bash
# Build the [lisa] pacman repo from the current git HEAD (Track L).
# Run on Arch (host or container) as an unprivileged user with
# base-devel + cargo installed. Output: a repo directory usable as
#   Server = file:///path/to/out
# Usage: build-packages.sh [outdir]
set -euo pipefail

root=$(git rev-parse --show-toplevel)
ver=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/libs/liblisa/Cargo.toml" | head -1)
out=${1:-"$root/os/repo-tools/out"}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

echo ">> lisa $ver -> $out"
git -C "$root" archive --prefix "lisa-$ver/" -o "$work/lisa-$ver.tar.gz" HEAD
cp "$root/os/packages/lisa/PKGBUILD" "$root/os/packages/lisa/lisa-inferenced.service" "$work/"

(cd "$work" && makepkg --noconfirm --force)

mkdir -p "$out"
cp "$work"/*.pkg.tar.* "$out/"
repo-add --new "$out/lisa.db.tar.gz" "$out"/*.pkg.tar.*

echo ">> repo ready: Server = file://$out"
