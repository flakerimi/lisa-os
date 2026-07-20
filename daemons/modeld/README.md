# lisa-modeld — model catalog & store

Spec: docs/PLAN.md §5.2 — read it before changing this component (CLAUDE.md rule 1).

Acquires, verifies, stores, and describes models; profiles hardware; recommends the tier lineup. The only Lisa component allowed network access for model traffic. Store is content-addressed (blake3 blobs + hardlinked refs) under /var/lib/lisa/models — dedupe across variants, survives OS updates, atomic swaps.

**M0 state:** Store implemented (add/list/verify/gc, pinned-hash ingest, dedupe) with tests; catalog TOML parsing against the seed in models/catalog; plain pinned download via ureq. M1 adds the D-Bus service, hardware profiler, TUF-style signed catalog, and delta/resumable downloads.
