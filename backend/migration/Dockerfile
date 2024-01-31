FROM rust:1.75.0-slim-buster AS builder

ARG ARCH
WORKDIR /complier

RUN apt update -y
RUN apt install musl-tools protobuf-compiler pkg-config openssl make -y

RUN cargo install just

RUN rustup target add ${ARCH}-unknown-linux-musl

WORKDIR /complier
COPY . .
RUN cargo install --target ${ARCH}-unknown-linux-musl --path backend/migration

FROM debian
COPY --from=builder /usr/local/cargo/bin/migration /
RUN apt install openssl -y
CMD ["/migration"]
