# AGENTS.md

## Architecture

- Single-crate workspace: `server/` (crate `zoo-agent-server`, binary `zooassistant`) + `web/` (Vite/React).
- The `agentzoo` framework is a git dependency tracking the `main` branch (`branch = "main"` in `Cargo.toml`, `github:hYuang/agentzoo#main` in `web/package.json`). It is NOT vendored in this repo. To pin a specific release, change both files to a tag, then `cargo update` + `npm install`.
- `AssistantApp` (server/src/lib.rs) implements `zoo_bootstrap::App`; `routes()` returns a plain axum `Router`. Write handlers/routes like a standalone axum app.
- The `web/dist` bundle is embedded into the binary at compile time via `rust-embed` (server/src/lib.rs:368). Rebuild the frontend whenever it changes, or the binary serves stale assets. If only `web/dist` changed, cargo may skip recompiling — touch `server/src/lib.rs` (or `cargo clean -p zoo-agent-server`) to force re-embedding.
- Workflow definition (stages, gates, artifacts, chat tasks) lives declaratively in `server/src/lib.rs`; shared tutor prompt is `server/prompts/assistant.md` (Chinese stage prompts are intentional).

## Build & run

- `make build` builds the frontend first, then the server, and copies the binary to `bin/zooassistant[.exe]`. Never run `cargo build` for a product build without building `web/` first.
- All cargo commands must include `--no-default-features` (the Makefile always passes it). Optional `zvec` feature: `make ZVEC=1 build`.
- `make test` = `cargo test --no-default-features`. Single test: `cargo test --no-default-features <name>`.
- `make run` = build + `bin/zooassistant web`.
- There is no lint/typecheck script. Frontend typecheck is `npm run build` in `web/` (runs `tsc --noEmit`); Rust formatting is `make fmt`.
- `web/dist/`, `bin/`, `target/` are gitignored build outputs — never commit them.
- Runtime state defaults to `~/.zooagent`; override with `ZOO_DATA_DIR`. `ZOO_HOST` defaults to `127.0.0.1` and `ZOO_PORT` accepts only `9211` or `6040`. Model config comes from `$ZOO_MODELS` or `./zooagent.json`.
- `zooagent.json` is local-only and gitignored (it contains endpoints/credentials). Use `zooagent.example.json` as the template.

## Release & distribution

- `.github/workflows/release.yml` builds five platform archives (Linux x86_64/arm64, Windows x86_64, macOS x86_64/arm64) and the multi-arch GHCR image on `v*` tags (or manually). It needs the `AGENTZOO_TOKEN` repo secret with read access to the private `hYuang/agentzoo` repo.
- `Dockerfile` is a multi-stage build; `AGENTZOO_TOKEN` is a BuildKit secret (`--mount=type=secret`) used only while fetching dependencies. Final image runs as `zoo` with `/data` as `ZOO_DATA_DIR` and `ZOO_HOST=0.0.0.0`. `docker-compose.yml` is the local run convenience file.
- Prebuilt releases/images are the supported path for users without framework access; source builds require git credentials for `hYuang/agentzoo`.

## Frontend dev

- `npm run dev` in `web/` starts Vite on port 5175, proxying API paths to the server at `127.0.0.1:9211` (see `web/vite.config.ts`). The server must be running for the dev UI to work.
- `@agentzoo/web-ui` is the framework's shared UI package (git dep); app-specific UI is in `web/src/assistant/`.

## Synapse Studio sync

- All Synapse code lives in `server/src/synapse/` (`mod.rs` core/client, `api.rs` routes) and `web/src/assistant/synapse/` (`api.ts`, `session.tsx`, `account.tsx`, module UI).
- Studio access goes through the gateway only: `/gateway/public/service/studio/v1/AuthService/*` and `/gateway/api/service/studio/v1/QuizService/*`. The gateway needs the `Studio.Auth.*` / `Studio.Quiz.*` routes from `gateway/backend/sql/dml.sql`; do not add endpoints to Studio.
- Auth is either an in-memory Studio login from the UI (`AuthService/GetPublicKey` + `Login`, RSA-OAEP) or env `SYNAPSE_TENANT_ID` plus `SYNAPSE_STUDIO_REFRESH_TOKEN`/`SYNAPSE_STUDIO_TOKEN`; optional `SYNAPSE_GATEWAY_URL` (default `https://mcode.cloud`). The Studio account must have quiz create permissions. Never persist the login password.
- The `knowledge` artifact (`knowledge-map.md`) is synced as **one** Studio knowledge point (`content` = full Markdown); every question links to that single id. Do not split it per question concept.
- Sync state is the run artifact `synapse` (`synapse.json`). Publishing has two modes: `add` creates a new knowledge point, paper, and questions and appends them to `knowledge_points`/`papers` (previous sets are kept and untouched); `overwrite` deletes every paper/question previously recorded for the run, then recreates them under the run's latest knowledge point (updated in place). The gateway exposes no knowledge-point delete API, so knowledge points created by earlier `add` publishes remain in Studio after an `overwrite`.

## Conventions

- License is PolyForm Noncommercial 1.0.0 (source-available; no commercial use without a separate license). Do not add MIT/Apache headers.
- The only CI is `.github/workflows/release.yml`; there is no pre-commit or task runner. The Makefile is the only local automation.
