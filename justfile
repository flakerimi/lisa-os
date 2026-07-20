# Lisa OS monorepo task runner (PLAN §9, Appendix D).

default: build

build:
    cargo build --workspace

test:
    cargo test --workspace

lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings

fmt:
    cargo fmt --all

# What CI runs on every PR.
ci: lint test

# End-to-end smoke: daemon up → streamed ask → health → daemon down.
smoke:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build -p lisa-inferenced -p lisa >/dev/null
    ./target/debug/lisa-inferenced & DAEMON=$!
    trap 'kill $DAEMON 2>/dev/null || true' EXIT
    sleep 1
    ./target/debug/lisa ask "write a haiku about entropy"
    curl -sf 127.0.0.1:7777/health >/dev/null && echo "health: ok"

# Build the immutable OS image (Track I). Linux only; normally CI's job.
image:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "$(uname)" != "Linux" ]; then
        echo "just image requires Linux (mkosi); CI builds it — see .github/workflows/nightly.yml" >&2
        exit 1
    fi
    mkosi --directory os/mkosi build

# Boot the built image in QEMU. Linux only.
vm:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "$(uname)" != "Linux" ]; then
        echo "just vm requires Linux (mkosi qemu)" >&2
        exit 1
    fi
    mkosi --directory os/mkosi qemu

# Track L: install/uninstall the Lisa Layer on stock Arch/Omarchy.
layer-install:
    bash os/layer/install.sh

layer-uninstall:
    bash os/layer/uninstall.sh
