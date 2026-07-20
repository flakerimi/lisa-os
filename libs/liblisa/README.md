# liblisa — the SDK core

Spec: docs/PLAN.md §5.6 — read it before changing this component (CLAUDE.md rule 1).

Make the right thing the easy thing: sessions and streaming, guided generation (JSON Schema → guaranteed-valid output), tool calling, scoped context retrieval, app memory, and the tasks API — Rust core → C ABI → GObject Introspection → Qt.

**M0 state:** grammar module: JSON Schema → GBNF for the M0 subset (objects, arrays, scalars, enum/const) with unit tests. The 1,000-sample validation gate against a real model is the M1 acceptance; sessions/tasks/memory land in M2.
