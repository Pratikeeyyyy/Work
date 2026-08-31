# syntax=docker/dockerfile:1

# ---- Build stage ----
FROM rust:1.97-bookworm AS build
WORKDIR /app

# Cache dependencies first (Cargo.toml + Cargo.lock before source).
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true
RUN rm -rf src

# Now copy the real source and rebuild (incremental).
COPY src ./src
RUN cargo build --release

# ---- Runtime stage ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/leadgen /usr/local/bin/leadgen

# Render injects a writable volume path via DATABASE_PATH; default to ./data
RUN mkdir -p /app/data
ENV DATABASE_PATH=/app/data/leadgen.db
EXPOSE 8080
CMD ["/usr/local/bin/leadgen"]
