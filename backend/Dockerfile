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

RUN sed -i '/mold/d' backend/Cargo.toml
RUN sed -i '/codegen-backend/d' backend/Cargo.toml

ARG ARCH=x86_64

RUN rustup target add ${ARCH}-unknown-linux-musl
RUN cargo install --no-default-features --target ${ARCH}-unknown-linux-musl --path backend

FROM scratch
# WORKDIR are used instead of mkdir
WORKDIR /database
WORKDIR /config
WORKDIR /
COPY --from=builder /usr/local/cargo/bin/backend /

CMD ["/backend"]
