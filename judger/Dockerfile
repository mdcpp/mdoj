FROM rust:1.75.0-slim-buster AS builder

ARG ARCH
WORKDIR /complier

RUN apt update -y
RUN apt install musl-tools protobuf-compiler pkg-config make -y

RUN cargo install just

RUN rustup target add ${ARCH}-unknown-linux-musl

WORKDIR /complier
COPY . .

RUN cargo install --target ${ARCH}-unknown-linux-musl --path judger

FROM scratch
WORKDIR /database
WORKDIR /config
WORKDIR /.temp
WORKDIR /
COPY --from=builder /usr/local/cargo/bin/judger /

COPY judger/nsjail-3.1 /

WORKDIR /plugins/rlua-54
COPY judger/plugins/rlua-54/rootfs .
COPY judger/plugins/rlua-54/spec.toml .

WORKDIR /
CMD ["/judger"]