FROM rust:1.74.0-slim-buster AS builder

ARG ARCH
WORKDIR /complier

RUN apt update -y
RUN apt install musl-tools protobuf-compiler -y

RUN cargo install just

RUN rustup target add ${ARCH}-unknown-linux-musl

WORKDIR /complier/proto
COPY backend.proto .
COPY judger.proto .

WORKDIR /complier/backend
COPY . .

RUN cargo install --target ${ARCH}-unknown-linux-musl --path .

FROM scratch
WORKDIR /database
WORKDIR /config
WORKDIR /cert
COPY ./*.pem .
WORKDIR /backend
COPY --from=builder /usr/local/cargo/bin/backend /

CMD ["/backend/backend"]
