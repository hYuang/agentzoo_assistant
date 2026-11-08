# agentzoo_assistant

Learning assistant built on the [agentzoo](https://github.com/hYuang/agentzoo) framework.

The project is source-available under the PolyForm Noncommercial License 1.0.0;
commercial use requires a separate license. See [License](#license).

## Layout

```
agentzoo_assistant/
├── server/          # app crate + product binary
│   ├── prompts/     # prompt assets
│   └── src/         # workflow/run engine, handlers, services, routes
├── web/             # Vite/React frontend
├── .github/workflows/release.yml  # release binaries + container image
├── Dockerfile
├── zooagent.example.json          # sample model/runtime config
├── Cargo.toml       # workspace: server
├── Cargo.lock
└── Makefile
```

The assistant app owns its workflow engine, HTTP routes, tools, frontend, and business state. It implements the framework's normal `zoo_bootstrap::App` trait; it does not use plugins.

## Framework dependency

The framework is consumed as a normal dependency. There is no sibling checkout or path dependency:

```toml
zoo-server = { git = "https://github.com/hYuang/agentzoo.git", branch = "main", default-features = false }
```

```json
{
  "dependencies": {
    "@agentzoo/web-ui": "github:hYuang/agentzoo#main"
  }
}
```

The dependencies track the framework's `main` branch. To pin a specific release, change the branch to a tag in `Cargo.toml` and `web/package.json`, then run `cargo update` and `npm install`.

`hYuang/agentzoo` is private. Building this app from source therefore needs read access to that repository (a PAT or deploy key); see [Build from source](#build-from-source). Prebuilt binaries and container images below do not.

## App integration

The app is composed like a normal library:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = zoo_agent_server::app();
    zoo_bootstrap::app::run(app).await
}
```

`AssistantApp` implements `zoo_bootstrap::App`. Its `routes()` method returns a normal axum `Router`, so handlers, services, extractors, middleware, and routes are written exactly like a standalone axum application.

## Synapse Studio sync

The Studio sync module (Chinese UI name: 智学工坊) publishes a generated practice set plus the full `knowledge-map.md` into the Synapse Studio quiz bank. Studio stores that file as a single knowledge point (its Markdown content), and all synced questions link to it. All Studio traffic goes through the gateway: the existing Studio APIs are exposed under `/gateway/public/service/studio/v1/AuthService/*` and `/gateway/api/service/studio/v1/QuizService/*`. No Studio-side changes are required; the gateway needs the corresponding `gateway_route` rows (`Studio.Auth.*` / `Studio.Quiz.*`) seeded from `gateway/backend/sql/dml.sql`.

Configure the assistant with:

```bash
SYNAPSE_GATEWAY_URL=https://mcode.cloud   # default
SYNAPSE_TENANT_ID=nebula
SYNAPSE_STUDIO_REFRESH_TOKEN=...           # preferred: refreshed automatically
# or SYNAPSE_STUDIO_TOKEN=...              # short-lived access token
```

Use an account that can create knowledge points, questions, and papers (the Studio admin role). You can also log in from the bottom-left Studio account panel with a tenant/account/password; the password is only used to obtain a token (`/gateway/public/service/studio/v1/AuthService/GetPublicKey` + `/Login`) and is not stored. The sync record is stored as `synapse.json` under the run's artifacts and shown in the workbench.

When a run has already been published, the module offers two modes: **Add a new set** creates a new knowledge point plus a new paper with fresh questions and keeps all earlier sets untouched, while **Replace previous set** deletes this run's previously synced papers and questions first, then republishes under the run's latest knowledge point. The gateway has no knowledge-point delete API, so knowledge points created by earlier **Add** publishes remain in Studio after a replace.

## Run a prebuilt release

Download the archive for your platform from
[Releases](https://github.com/hYuang/agentzoo_assistant/releases), unpack it, and run:

```bash
cp zooagent.example.json zooagent.json   # optional: point at your own model endpoints
./zooassistant                           # Windows: .\zooassistant.exe
```

Open <http://127.0.0.1:9211>. State (SQLite, logs, secrets) is stored in `~/.zooagent`
by default. `zooagent.json` is optional; without it the built-in model catalog is used.

Runtime configuration:

| Variable | Default | Purpose |
| --- | --- | --- |
| `ZOO_HOST` | `127.0.0.1` | Bind address. Use `0.0.0.0` inside containers. |
| `ZOO_PORT` | `9211` | Bind port (`9211` or `6040`). |
| `ZOO_DATA_DIR` | `~/.zooagent` | Persistence directory (SQLite, logs, secrets). |
| `ZOO_MODELS` | `./zooagent.json` | Model/config file path. |
| `ZOO_NO_OPEN` | unset | Set to `1` to skip launching a browser. |
| `RUST_LOG` | `info` | Log filter. |
| `SYNAPSE_*` | unset | Studio sync; see below. |

## Docker

```bash
# Prebuilt image (no framework access needed)
docker run -d --name agentzoo-assistant \
  -p 127.0.0.1:9211:9211 \
  -v agentzoo-data:/data \
  ghcr.io/hyuang/agentzoo_assistant:latest
```

Mount a config file when you need custom model endpoints:

```bash
docker run -d --name agentzoo-assistant \
  -p 127.0.0.1:9211:9211 \
  -v agentzoo-data:/data \
  -v "$PWD/zooagent.json:/data/zooagent.json:ro" \
  ghcr.io/hyuang/agentzoo_assistant:latest
```

Or use Compose: `docker compose up -d` (see `docker-compose.yml`).

Build the image yourself (requires a token with read access to `hYuang/agentzoo`):

```bash
printf '%s' "$AGENTZOO_TOKEN" > /tmp/agentzoo-token
docker build --secret id=AGENTZOO_TOKEN,src=/tmp/agentzoo-token -t agentzoo-assistant .
rm -f /tmp/agentzoo-token
```

The token is mounted only while dependencies are fetched; it is not baked into the
final image or exported layer caches.
After the first push, set the GHCR package visibility to **Public** in the package settings;
otherwise anonymous `docker pull` fails and users must `docker login ghcr.io` first.

## Build from source

Source builds require read access to the private `hYuang/agentzoo` repository. Configure
git to authenticate (for example `git config --global url."https://x-access-token:${AGENTZOO_TOKEN}@github.com/".insteadOf "https://github.com/"`), then:

```bash
# Build the frontend, then the server binary + bin/zooassistant[.exe]
make build

# Run
make run          # or: bin/zooassistant web

# Tests
make test

# Release
make release
```

The frontend is embedded into the server binary at build time, so `web/` must be built before the Rust server.

## Releases

Pushing a `v*` tag builds five archives (Linux x86_64/arm64, Windows x86_64,
macOS x86_64/arm64) and publishes a multi-arch (`amd64`/`arm64`) container image to GHCR:

```bash
git tag v1.1.0
git push origin v1.1.0
```

The workflow needs a repository secret `AGENTZOO_TOKEN` with read-only access to
`hYuang/agentzoo`. You can also run the workflow manually; artifacts are then uploaded
without creating a release.

## Publishing and framework code

This repository contains only assistant application code. It does not vendor or copy the `agentzoo` framework source:

- Rust dependencies are resolved from the framework git repository.
- `Cargo.lock` contains only dependency URLs, tags, and commit hashes.
- `web/package-lock.json` contains the npm dependency URL and commit.
- `web/dist/`, `bin/`, and `target/` are build outputs and are gitignored. The generated frontend bundle and binary are not committed.

Prebuilt releases and the GHCR image are the supported way to use the app without
framework access. Keep `zooagent.json` untracked: it contains local model endpoints and
possibly credentials. Use `zooagent.example.json` as the template.

## License

Source-available under the [PolyForm Noncommercial License 1.0.0](LICENSE): personal,
research, educational, and other noncommercial use is permitted. Commercial use requires a
separate written license from the copyright holder. See [LICENSE](LICENSE).

The `agentzoo` framework is a separate dependency and remains under its own license.
