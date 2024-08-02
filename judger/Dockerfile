FROM rust:1.75.0-slim-bullseye AS builder

RUN apt update -y \
    && apt install musl-tools protobuf-compiler pkg-config make -y

RUN cargo install just

RUN --mount=type=cache,target=~/.cargo/bin/
RUN --mount=type=cache,target=~/.cargo/registry/index/
RUN --mount=type=cache,target=~/.cargo/registry/cache/
RUN --mount=type=cache,target=~/.cargo/git/db/

WORKDIR /compiler
COPY . .
RUN --mount=type=cache,target=target

ARG ARCH=x86_64

RUN rustup target add ${ARCH}-unknown-linux-musl
RUN cargo install --target ${ARCH}-unknown-linux-musl --path judger

FROM alpine:3.20
WORKDIR /plugins
WORKDIR /config
WORKDIR /

RUN apk add --no-cache fuse3

COPY --from=builder /usr/local/cargo/bin/judger /

COPY judger/nsjail-3.1 /

CMD ["/judger"]