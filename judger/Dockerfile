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
COPY --from=builder /usr/local/cargo/bin/judger /
COPY --from=builder /environment/.temp /
COPY nsjail-3.1 /
COPY plugins /

CMD ["/judger"]