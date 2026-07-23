# lisa-contextd — context fabric

Spec: docs/PLAN.md §5.3. Milestone: M3.

System-wide personal context index (files, mail, calendar, chat, screen —
each individually consented) plus per-app durable memory. SQLite + FTS5
(+ vectors), every retrieval ledgered with provenance tags and per-chunk
scope ACLs.

## Status: M3 core + hybrid + scoped-ACL (landed)

Implemented and unit-tested (macOS + Linux, no daemon required):

- **File ingestion** (`index.rs`) — walk → text-filter → ~1 KiB
  paragraph chunks → FTS5, incremental (mtime + blake3 hash skip
  unchanged, atomic reindex of changed). Every chunk carries a
  provenance tag.
- **Lexical retrieval** (`index.rs::search`) — FTS5 bm25, best-first,
  with provenance + snippet.
- **Hybrid retrieval** (`embed.rs`) — per-chunk embeddings + BM25×cosine
  blend over FTS-prefiltered candidates (sqlite-vec at scale is the later
  optimization). `embed_pending`, `search_hybrid`.
- **Per-app memory** (`memory.rs`) — namespace-isolated key/value with
  zero-residual wipe (an app never reads another's namespace).
- **Scoped-ACL retrieval** (`acl.rs`) — maps a granted portal scope to
  the provenance it may read and filters *at the query*, so a
  disallowed-provenance chunk can't leak through ranking even when it
  ranks best. Deny-by-default on empty/unknown scopes. `search_scoped`;
  ACL-leak + fuzz tests assert **0 cross-scope leaks** (§5.3 acceptance).
  `add_document` ingests non-file (mail/screen/web) provenance.

CLI: `lisa context index [--embed]`, `lisa context search [--hybrid]
[--scope <scope>]` (scoped searches ledger as `context.search.scoped`).

## Left for M3 completion

Live sources + watchers (file/mail/calendar ingestion daemons),
sqlite-vec at scale, encryption-at-rest (keyring), the D-Bus/portal
serving surface, and the Settings › Intelligence panel. Provenance is
load-bearing (CLAUDE.md rule 6): ingestion never lets an untrusted caller
forge a provenance tag — real mail/screen chunks arrive via
portal-mediated sources, not a raw CLI flag.

Prior art: [cognee evaluation](../../docs/notes/cognee-evaluation.md) —
knowledge-graph memory platform; not the substrate (Python, multi-engine),
but a flagship M5 MCP tenant and a design reference for the M3
entity-graph question.
