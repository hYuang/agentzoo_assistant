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

## Conventions

- License is proprietary (all rights reserved). Do not add MIT/Apache headers or publish artifacts.
- No CI, pre-commit, or task runner is configured; the Makefile is the only automation.
