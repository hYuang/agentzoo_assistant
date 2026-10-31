# agentzoo_assistant

Learning assistant built on the [agentzoo](https://github.com/hYuang/agentzoo) framework.

## Layout

```
agentzoo_assistant/
├── server/          # app crate + product binary
│   ├── prompts/     # prompt assets
│   └── src/         # workflow/run engine, handlers, services, routes
├── web/             # Vite/React frontend
├── Cargo.toml       # workspace: server
├── Cargo.lock
└── Makefile
```

The assistant app owns its workflow engine, HTTP routes, tools, frontend, and business state. It implements the framework's normal `zoo_bootstrap::App` trait; it does not use plugins.

## Framework dependency

The framework is consumed as a normal dependency. There is no sibling checkout or path dependency:

```toml
zoo-server = { git = "https://github.com/hYuang/agentzoo.git", tag = "v1.3.0", default-features = false }
```

```json
{
  "dependencies": {
    "@agentzoo/web-ui": "git+https://github.com/hYuang/agentzoo.git#v1.3.0"
  }
}
```

To upgrade, change the tag in `Cargo.toml` and `web/package.json`, then run `cargo update` and `npm install`.

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

## Build

```bash
# Build the frontend, then the server binary + bin/zooagent[.exe]
make build

# Run
make run          # or: bin/zooagent web

# Tests
make test

# Release
make release
```

The frontend is embedded into the server binary at build time, so `web/` must be built before the Rust server.

## Publishing and framework code

This repository contains only assistant application code. It does not vendor or copy the `agentzoo` framework source:

- Rust dependencies are resolved from the framework git repository.
- `Cargo.lock` contains only dependency URLs, tags, and commit hashes.
- `web/package-lock.json` contains the npm dependency URL and commit.
- `web/dist/`, `bin/`, and `target/` are build outputs and are gitignored. The generated frontend bundle and binary are not committed.

One practical requirement for publishing: build users need read access to the framework repository. If this app is public while `agentzoo` remains private, external users cannot install the dependencies. Either make the framework repository public or publish the framework crates/package to a registry before publishing this app.

Before publishing, review `zooagent.json` / environment files for secrets. The current tree does not commit model API keys.

## License

Proprietary, all rights reserved. No use, copying, modification, distribution, hosting, or commercial use is permitted without prior written permission. See [LICENSE](LICENSE).

The `agentzoo` framework is a separate dependency and remains under its own license.
