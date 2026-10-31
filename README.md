# agentzoo_assistant

A self-contained product app built on the [agentzoo](https://github.com/hYuang/agentzoo) framework.

## Layout

```
agentzoo_assistant/
├── server/          # app crate + product binary
│   ├── src/         # workflow/run engine, routes, app extension
│   ├── prompts/     # prompt assets
│   └── tests/
├── web/             # Vite/React frontend (host + assistant dashboard)
├── Cargo.toml       # workspace: server
├── Cargo.lock
└── Makefile
```

The framework is consumed as a normal dependency; there is no sibling checkout or path dependency:

```toml
zoo-server = { git = "https://github.com/hYuang/agentzoo.git", tag = "v1.2.1", default-features = false }
```

```json
{
  "dependencies": {
    "@agentzoo/web-ui": "git+https://github.com/hYuang/agentzoo.git#v1.2.1"
  }
}
```

To upgrade, change the tag in `Cargo.toml` and `web/package.json`, then run `cargo update` and `npm install`.

## Build

```bash
# Debug binary + web assets
make build

# Run the web app
make run          # or: bin/zooagent web

# Tests
make test

# Release binary
make release
```

The frontend is embedded into the server binary at build time, so build `web/` before compiling the server.
