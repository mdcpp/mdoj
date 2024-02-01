FROM rust:1.75.0-slim-buster AS builder

ARG ARCH
WORKDIR /complier

RUN apt update -y
RUN apt install musl-tools protobuf-compiler pkg-config make -y

RUN cargo install just


WORKDIR /complier
COPY . .

RUN rustup target add ${ARCH}-unknown-linux-musl

RUN cargo install --target ${ARCH}-unknown-linux-musl --path backend/migration

FROM scratch
COPY --from=builder /usr/local/cargo/bin/migration /

CMD ["/migration"]
