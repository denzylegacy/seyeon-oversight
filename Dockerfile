FROM rust:1.86-slim as builder
WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY apps/oversight/Cargo.toml ./apps/oversight/
COPY crates/seyeon_cryptocompare/Cargo.toml ./crates/seyeon_cryptocompare/
COPY crates/seyeon_email/Cargo.toml ./crates/seyeon_email/
COPY crates/seyeon_rapidapi/Cargo.toml ./crates/seyeon_rapidapi/
COPY crates/seyeon_redis/Cargo.toml ./crates/seyeon_redis/
COPY crates/seyeon_shared_models/Cargo.toml ./crates/seyeon_shared_models/
COPY crates/seyeon_trading_engine/Cargo.toml ./crates/seyeon_trading_engine/

RUN mkdir -p apps/oversight/src \
    crates/seyeon_cryptocompare/src \
    crates/seyeon_email/src \
    crates/seyeon_rapidapi/src \
    crates/seyeon_redis/src \
    crates/seyeon_shared_models/src \
    crates/seyeon_trading_engine/src

RUN for dir in crates/seyeon_cryptocompare \
    crates/seyeon_email \
    crates/seyeon_rapidapi \
    crates/seyeon_redis \
    crates/seyeon_shared_models \
    crates/seyeon_trading_engine; \
    do echo "// placeholder" > $dir/src/lib.rs; done

RUN echo "fn main() {}" > apps/oversight/src/main.rs

RUN cargo build --release --bin oversight

COPY . .

RUN cargo build --release --bin oversight

FROM debian:stable-slim
WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/oversight .

COPY --from=builder /usr/src/app/apps/oversight/assets ./assets

CMD ["./oversight"]