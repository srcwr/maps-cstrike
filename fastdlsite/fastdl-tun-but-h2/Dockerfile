FROM rust:latest AS builder

WORKDIR /usr/src/app
COPY . .
# Will build and cache the binary and dependent crates in release mode
RUN --mount=type=cache,target=/usr/local/cargo,from=rust:latest,source=/usr/local/cargo \
    --mount=type=cache,target=target \
    cargo build --release && mv ./target/release/fastdl-tun-but-h2 ./fastdl-tun-but-h2


FROM debian:stable-slim

RUN useradd -ms /bin/bash app
USER app
WORKDIR /app

COPY --from=builder /usr/src/app/fastdl-tun-but-h2 /app/fastdl-tun-but-h2

ENTRYPOINT ["./fastdl-tun-but-h2"]
