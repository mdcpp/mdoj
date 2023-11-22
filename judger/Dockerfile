FROM rust:1.74.0-slim-buster AS builder

ARG ARCH
WORKDIR /complier

RUN apt update -y
RUN apt install musl-tools protobuf-compiler -y

RUN cargo install just

RUN rustup target add ${ARCH}-unknown-linux-musl

WORKDIR /complier/proto
COPY judger.proto .

WORKDIR /complier/judger
COPY . .

RUN cargo install --target ${ARCH}-unknown-linux-musl --path .

WORKDIR /environment
RUN mkdir -p .temp

FROM scratch
WORKDIR /
COPY --from=builder /usr/local/cargo/bin/jurdger /
COPY --from=builder /environment/.temp /
COPY nsjail-3.1 /
# don't load plugins, should be mounted manually in runtime
# COPY plugins /

CMD ["/judger"]