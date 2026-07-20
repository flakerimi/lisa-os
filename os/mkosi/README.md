# os/mkosi — Track I image build

Spec: PLAN §3 (immutability stack), §6 (pipeline). ADR-0001.

Target state: signed UKI + `systemd-repart` partitions, A/B roots via
`systemd-sysupdate`, dm-verity base, boot-counting rollback, LUKS2+TPM2.
M0 acceptance: fresh clone → `just image` → bootable qcow2; update →
rollback demonstrated in the QEMU test.

Status: **scaffold.** `mkosi.conf` is a minimal bootable Arch profile;
`mkosi.repart/` sketches the partition set (single root for now — the
A/B pair + verity partitions are the next backlog item). Nightly CI
validates the profile parses (`mkosi summary`); the QEMU+swtpm boot test
is the M0 gate.

Requires Linux; on macOS dev hosts this directory is CI-only.
