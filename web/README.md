# web/ — Lisa OS sites

Two static sites, each a tiny nginx container deployed via Basepod (`bp`).
See `docs/WEB_PLAN.md` for the strategy.

- `app/` → **lisa-app.common.al** — marketing landing ("A computer that's
  genuinely yours to think with"). Later: `lisaos.app`.
- `dev/` → **lisa-dev.common.al** — docs / install / SDK. Later:
  `lisaos.dev` (this will move to a generated site over `docs/`).

Both use the violet wordmark brand (`branding/`), are light/dark aware,
and are self-contained (inline CSS + the wordmark paths).

## Deploy

```sh
cd web/app && bp deploy      # → lisa-app on bp.common.al
cd web/dev && bp deploy      # → lisa-dev on bp.common.al
```

`bp` needs a clean git tree (commit first). Content lives in `public/`;
the `Dockerfile` serves it with nginx on :80.
