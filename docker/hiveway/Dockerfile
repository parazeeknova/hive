# syntax=docker/dockerfile:1
FROM rust:slim-trixie@sha256:b8e062b1c3a80183df83d9ba187cadd2b5ab84bc78d453dbfeb2fccd5f982247 AS build
RUN apt-get update && apt-get install -y --no-install-recommends clang cmake pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace
COPY rust-toolchain.toml ./
RUN rustup show active-toolchain
COPY Cargo.toml Cargo.lock ./
COPY apps/hiveway apps/hiveway
ARG HIVE_BUILD_REVISION=development
ENV HIVE_BUILD_REVISION=$HIVE_BUILD_REVISION
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/workspace/target cargo build --locked --release -p hiveway && cp target/release/hiveway /usr/local/bin/hiveway

FROM debian:trixie-slim@sha256:f6b08b09a05725f9431502d3b5714f5c4576e82c1a84fa1debe0c24a7e0c3ec4 AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/* && groupadd --gid 10001 hive && useradd --uid 10001 --gid hive --no-create-home hive
ENV HIVE_BIND_ADDRESS=0.0.0.0:3001 RUST_LOG=info
COPY --from=build /usr/local/bin/hiveway /usr/local/bin/hiveway
USER hive
EXPOSE 3001
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=15s --timeout=5s --start-period=60s --retries=4 CMD curl --fail --silent http://127.0.0.1:3001/api/ready || exit 1
ENTRYPOINT ["/usr/local/bin/hiveway"]
