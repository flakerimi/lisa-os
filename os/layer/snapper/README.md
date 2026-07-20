# os/layer/snapper — Track L rollback

Omarchy's proven combo, adopted wholesale (PLAN §3 Track L, Appendix E
rule 2): Btrfs + Snapper pre-update snapshot before every layer update,
snapshots selectable from the Limine boot menu.

Their paid-for lessons, kept as hard rules:

- Snapshot `/` **only**, never `/home` (space blowup).
- Btrfs quotas **off** (performance).
- Restore flow returns `/` to the pre-update snapshot without touching
  user data.

Lands with the first real `install.sh` (M0→M1 backlog): a pacman
pre-transaction hook + tuned snapper config + Limine sync, per Appendix
D item `os/layer/snapper/`.
