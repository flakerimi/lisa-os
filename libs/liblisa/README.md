# liblisa — the SDK core

Spec: docs/PLAN.md §5.6 — read it before changing this component (CLAUDE.md rule 1).

Make the right thing the easy thing: sessions and streaming, guided generation (JSON Schema → guaranteed-valid output), tool calling, scoped context retrieval, app memory, and the tasks API — Rust core → C ABI → GObject Introspection → Qt.

**M0 state:** grammar module: JSON Schema → GBNF (objects, arrays, scalars, enum/const, plus structural bounds: minItems/maxItems, minLength/maxLength — bounded repetition keeps small models from spiraling in constrained loops). Truth-tested against llama.cpp's sampler via lisa-inferenced; tests/e2e/guided-validation.sh is the sampled acceptance gate. Sessions/tasks/memory land in M2.
