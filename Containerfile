# Multi-stage Rust build for oa-telemetry-server
# Usage: podman build -f rust/Containerfile -t oa-telemetry-server rust/

FROM rust:1.93 AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
COPY migrations/ migrations/

# Build release binary
RUN cargo build --release --bin oa-telemetry-server

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/oa-telemetry-server /usr/local/bin/

ENV PORT=8007
EXPOSE 8007

CMD ["oa-telemetry-server"]
