# -------- Build stage --------
FROM rust:1.95 as builder

RUN rustup toolchain install nightly

WORKDIR /app

RUN rustup override set nightly

COPY ./crates/jlh_maps_core_service ./crates/jlh_maps_core_service

WORKDIR /app/crates/jlh_maps_core_service

RUN cargo build --package jlh_maps_core_service --release --bin run_server
RUN ls target/release

# -------- Runtime stage --------
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN find . -type f
COPY --from=builder /app/crates/jlh_maps_core_service/target/release/run_server /usr/local/bin/run_server

ENTRYPOINT ["run_server"]