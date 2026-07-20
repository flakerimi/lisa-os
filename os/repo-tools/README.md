# os/repo-tools — pinned snapshot mirror + custom repo

Spec: PLAN §3 ("packaging economics"), §6. We control when the Arch base
moves, like SteamOS's `holo` repo: the image and layer both build
against a **pinned snapshot** of Arch (snapshots served by the Arch
Linux Archive, archive.archlinux.org), plus our own small signed `[lisa]`
repo (~100–200 packages).

Backlog (Appendix D):

- `snapshot.sh` — record/advance the pinned snapshot date; snapshot
  advances only at channel promotion after a soak (PLAN §6).
- `repo-add` tooling for the `[lisa]` repo with signing.
- CI wiring so both mkosi (Track I) and the layer (Track L) resolve
  packages from the same snapshot.

Status: **empty scaffold** — first M0→M1 backlog item after the boot
test.
