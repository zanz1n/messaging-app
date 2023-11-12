FROM rust:1.73-bookworm AS builder

RUN apt-get update -y
RUN export DEBIAN_FRONTEND=noninteractive
RUN apt-get install -y apt-utils && \
    apt-get install -y \
    build-essential \
    musl-dev \
    musl-tools
RUN apt-get clean && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app
COPY . /app
RUN cargo build --release --features production

FROM gcr.io/distroless/static
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/messaging-app /server
CMD [ "/server" ]
