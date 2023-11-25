FROM rust:1.74.0-slim-buster AS builder

ARG ARCH
WORKDIR /complier

RUN apt update -y
RUN apt install musl-tools -y

RUN rustup target add ${ARCH}-unknown-linux-musl

WORKDIR /complier/rlua-54
COPY . .

RUN cargo install --target ${ARCH}-unknown-linux-musl --path .

FROM scratch
WORKDIR /
COPY --from=builder /usr/local/cargo/bin/rlua-54 /

CMD ["/rlua-54/rlua-54"]
