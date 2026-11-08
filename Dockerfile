# syntax=docker/dockerfile:1
#
# Build the assistant image without exposing the private agentzoo framework:
#
#   docker build --secret id=AGENTZOO_TOKEN,src=/path/to/token -t agentzoo-assistant .
#
# The secret must contain a GitHub token with read access to
# https://github.com/hYuang/agentzoo. It is mounted only while dependencies are
# fetched and is not written to the final image or to exported layer caches.

FROM node:20-bookworm-slim AS web
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN --mount=type=secret,id=AGENTZOO_TOKEN \
    TOKEN="$(cat /run/secrets/AGENTZOO_TOKEN)" \
    && git config --global --add url."https://x-access-token:${TOKEN}@github.com/".insteadOf "https://github.com/" \
    && git config --global --add url."https://x-access-token:${TOKEN}@github.com/".insteadOf "ssh://git@github.com/" \
    && git config --global --add url."https://x-access-token:${TOKEN}@github.com/".insteadOf "git@github.com:" \
    && npm ci --no-audit --no-fund \
    && rm -f /root/.gitconfig
COPY web/ ./
RUN npm run build

FROM rust:1-bookworm AS server
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY server/ ./server/
COPY --from=web /app/web/dist ./web/dist
RUN --mount=type=secret,id=AGENTZOO_TOKEN \
    TOKEN="$(cat /run/secrets/AGENTZOO_TOKEN)" \
    && git config --global --add url."https://x-access-token:${TOKEN}@github.com/".insteadOf "https://github.com/" \
    && git config --global --add url."https://x-access-token:${TOKEN}@github.com/".insteadOf "ssh://git@github.com/" \
    && git config --global --add url."https://x-access-token:${TOKEN}@github.com/".insteadOf "git@github.com:" \
    && cargo build --locked --release --no-default-features --bin zooassistant \
    && rm -f /root/.gitconfig \
    && rm -rf /usr/local/cargo/git

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --home-dir /data --shell /usr/sbin/nologin zoo
COPY --from=server /app/target/release/zooassistant /usr/local/bin/zooassistant
ENV ZOO_HOST=0.0.0.0 \
    ZOO_PORT=9211 \
    ZOO_DATA_DIR=/data \
    ZOO_NO_OPEN=1 \
    RUST_LOG=info
EXPOSE 9211
VOLUME ["/data"]
USER zoo
WORKDIR /data
ENTRYPOINT ["zooassistant"]
