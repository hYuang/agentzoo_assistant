# AGENTS.md

## Architecture

- Single-crate workspace: `server/` (crate `zoo-agent-server`, binary `zooagent`) + `web/` (Vite/React).
- The `agentzoo` framework is a git dependency pinned to a tag (`v1.3.0` in `Cargo.toml` and `web/package.json`). It is NOT vendored in this repo. To upgrade: change the tag in both files, then `cargo update` + `npm install`.
- `AssistantApp` (server/src/lib.rs) implements `zoo_bootstrap::App`; `routes()` returns a plain axum `Router`. Write handlers/routes like a standalone axum app.
- The `web/dist` bundle is embedded into the binary at compile time via `rust-embed` (server/src/lib.rs:472). Rebuild the frontend whenever it changes, or the binary serves stale assets.
- Workflow definition (stages, gates, artifacts, chat tasks) lives declaratively in `server/src/lib.rs`; shared tutor prompt is `server/prompts/assistant.md` (Chinese stage prompts are intentional).

## Build & run

- `make build` builds the frontend first, then the server, and copies the binary to `bin/zooagent[.exe]`. Never run `cargo build` for a product build without building `web/` first.
- All cargo commands must include `--no-default-features` (the Makefile always passes it). Optional `zvec` feature: `make ZVEC=1 build`.
- `make test` = `cargo test --no-default-features`. Single test: `cargo test --no-default-features <name>`.
- `make run` = build + `bin/zooagent web`.
- There is no lint/typecheck script. Frontend typecheck is `npm run build` in `web/` (runs `tsc --noEmit`); Rust formatting is `make fmt`.
- `web/dist/`, `bin/`, `target/` are gitignored build outputs — never commit them.

## Frontend dev

- `npm run dev` in `web/` starts Vite on port 5175, proxying API paths to the server at `127.0.0.1:9211` (see `web/vite.config.ts`). The server must be running for the dev UI to work.
- `@agentzoo/web-ui` is the framework's shared UI package (git dep); app-specific UI is in `web/src/assistant/`.

## Synapse Studio sync

- All Synapse code lives in `server/src/synapse/` (`mod.rs` core/client, `api.rs` routes) and `web/src/assistant/synapse/` (`api.ts`, `session.tsx`, `account.tsx`, module UI).
- Studio access goes through the gateway only: `/gateway/public/service/studio/v1/AuthService/*` and `/gateway/api/service/studio/v1/QuizService/*`. The gateway needs the `Studio.Auth.*` / `Studio.Quiz.*` routes from `gateway/backend/sql/dml.sql`; do not add endpoints to Studio.
- Auth is either an in-memory Studio login from the UI (`AuthService/GetPublicKey` + `Login`, RSA-OAEP) or env `SYNAPSE_TENANT_ID` plus `SYNAPSE_STUDIO_REFRESH_TOKEN`/`SYNAPSE_STUDIO_TOKEN`; optional `SYNAPSE_GATEWAY_URL` (default `http://127.0.0.1:9000`). The Studio account must have quiz create permissions. Never persist the login password.
- The `knowledge` artifact (`knowledge-map.md`) is synced as **one** Studio knowledge point (`content` = full Markdown); every question links to that single id. Do not split it per question concept.
- Sync state is the run artifact `synapse` (`synapse.json`). Re-publishing updates the knowledge point and deletes/recreates the previous paper/questions through the same gateway APIs.

## Conventions

- License is proprietary (all rights reserved). Do not add MIT/Apache headers or publish artifacts.
- No CI, pre-commit, or task runner is configured; the Makefile is the only automation.
