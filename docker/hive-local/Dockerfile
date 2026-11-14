FROM rust:slim-trixie@sha256:b8e062b1c3a80183df83d9ba187cadd2b5ab84bc78d453dbfeb2fccd5f982247 AS build
RUN apt-get update && apt-get install -y --no-install-recommends clang cmake pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /source
COPY rust-toolchain.toml ./
RUN rustup show active-toolchain
COPY Cargo.toml Cargo.lock ./
COPY apps/hiveway apps/hiveway
COPY apps/hive-local apps/hive-local
RUN cargo build --locked --release -p hive-local

FROM ghcr.io/astral-sh/uv:latest@sha256:5cc1805e2426484abca4ad46ea783bab281fdb225fd82d8ce6d66a81254bac85 AS uv
FROM debian:trixie-slim@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4
COPY --from=uv /uv /uvx /usr/local/bin/
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates python3 procps && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home worker
COPY --from=build /source/target/release/hive-local /usr/local/bin/hive-local
# Only the Rust supervisor reads the root-owned connector key. Every upstream
# and package preparation process drops to uid/gid 10001 before starting.
USER 0:0
ENV UV_CACHE_DIR=/tmp/uv-cache HOME=/tmp/home
ENTRYPOINT ["hive-local"]
CMD ["run", "--config", "/run/secrets/hive-local.json"]
