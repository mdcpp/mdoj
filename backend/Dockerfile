FROM rust:1.74.0-slim-buster AS builder

ARG ARCH
WORKDIR /complier

RUN apt update -y
RUN apt install musl-tools protobuf-compiler pkg-config libssl-dev -y

RUN cargo install just

RUN rustup target add ${ARCH}-unknown-linux-musl

WORKDIR /complier
COPY . .
RUN OPENSSL_STATIC=1 OPENSSL_DIR=/usr/include/openssl cargo install --target ${ARCH}-unknown-linux-musl --path backend

FROM scratch
WORKDIR /database
WORKDIR /config
WORKDIR /cert
WORKDIR /backend
COPY --from=builder /usr/local/cargo/bin/backend /backend

CMD ["/backend/backend"]
