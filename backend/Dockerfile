FROM rust:1.75.0-slim-buster AS builder

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

RUN sed -i '/mold/d' backend/Cargo.toml
RUN sed -i '/codegen-backend/d' backend/Cargo.toml

ARG ARCH

RUN rustup target add ${ARCH}-unknown-linux-musl
RUN cargo install --features release --target ${ARCH}-unknown-linux-musl --path backend

FROM scratch
# WORKDIR are used instead of mkdir
WORKDIR /database
WORKDIR /config
WORKDIR /
COPY --from=builder /usr/local/cargo/bin/backend /

CMD ["/backend"]
