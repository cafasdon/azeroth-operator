# Immutable official Rust and Debian manifests, resolved 2026-11-03.
FROM rust:1.98.1-slim-bookworm@sha256:ff521445a372125ed4f76e1453a1f8098f2d05332d1601d30db1c1f62757e730 AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY clients ./clients
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked && \
    cp target/release/azeroth-operator /azeroth-operator

FROM debian:bookworm-slim@sha256:f3034a6ec3c1205360777c4aae76234998866ad18806ae62b63a3f84ccad782b
COPY --from=build /azeroth-operator /usr/local/bin/azeroth-operator
RUN mkdir /data && chown 10006:10006 /data
USER 10006:10006
ENV AZEROTH_DATA_DIR=/data
ENTRYPOINT ["/usr/local/bin/azeroth-operator"]
