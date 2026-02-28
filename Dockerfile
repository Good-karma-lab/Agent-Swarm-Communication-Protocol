# World Wide Swarm — wws-connector
# Stage 1: Build
FROM rust:1.75-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin wws-connector

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/wws-connector /usr/local/bin/wws-connector
RUN useradd -m -u 1000 wws
USER wws
WORKDIR /home/wws
ENTRYPOINT ["wws-connector"]
