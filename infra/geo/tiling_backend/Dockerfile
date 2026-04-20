# -------- Build stage --------
FROM rust:1.95 as builder

RUN rustup toolchain install nightly

WORKDIR /app

RUN rustup override set nightly

COPY ./crates/backend ./crates/backend
COPY ./crates/backend_model ./crates/backend_model
COPY ./crates/utilities ./crates/utilities

WORKDIR /app/crates/backend

RUN cargo build --package backend --release --bin run_earth_tiling_service
RUN ls target/release

# -------- Runtime stage --------
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN find . -type f
COPY --from=builder /app/crates/backend/target/release/run_earth_tiling_service /usr/local/bin/run_earth_tiling_service

EXPOSE 8080

CMD ["run_earth_tiling_service"]