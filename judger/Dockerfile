FROM rust:1.75.0-slim-buster AS builder

ARG ARCH
WORKDIR /complier

RUN apt update -y
RUN apt install musl-tools protobuf-compiler pkg-config make -y

RUN cargo install just

RUN --mount=type=cache,target=~/.cargo/bin/
RUN --mount=type=cache,target=~/.cargo/registry/index/
RUN --mount=type=cache,target=~/.cargo/registry/cache/
RUN --mount=type=cache,target=~/.cargo/git/db/

WORKDIR /complier

COPY . .
RUN --mount=type=cache,target=target

RUN rustup target add ${ARCH}-unknown-linux-musl
RUN cargo install --target ${ARCH}-unknown-linux-musl --path judger

FROM scratch
WORKDIR /config
WORKDIR /.temp
WORKDIR /
COPY --from=builder /usr/local/cargo/bin/judger /

COPY judger/nsjail-3.1 /

WORKDIR /
CMD ["/judger"]