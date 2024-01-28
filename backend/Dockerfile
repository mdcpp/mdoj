FROM rust:1.75.0-slim-buster AS builder

ARG ARCH
WORKDIR /complier

RUN apt update -y
RUN apt install musl-tools protobuf-compiler pkg-config make -y

RUN cargo install just

RUN rustup target add ${ARCH}-unknown-linux-musl

WORKDIR /complier
COPY . .
RUN cargo install --target ${ARCH}-unknown-linux-musl --path backend

FROM scratch
# WORKDIR are used instead of mkdir
WORKDIR /database
WORKDIR /config
COPY --from=builder /usr/local/cargo/bin/backend /

CMD ["/backend"]
