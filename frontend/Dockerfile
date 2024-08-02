FROM rustlang/rust:nightly-bullseye AS builder

RUN apt update -y \
    && apt install musl-tools protobuf-compiler pkg-config make -y

RUN cargo install just cargo-leptos

RUN rustup target add wasm32-unknown-unknown

RUN --mount=type=cache,target=~/.cargo/bin/
RUN --mount=type=cache,target=~/.cargo/registry/index/
RUN --mount=type=cache,target=~/.cargo/registry/cache/
RUN --mount=type=cache,target=~/.cargo/git/db/

WORKDIR /compiler
COPY . .
RUN --mount=type=cache,target=target

RUN mkdir -p .cargo \
    && echo "[unstable]\nbuild-std = [\"std\", \"panic_abort\", \"core\", \"alloc\"]\nbuild-std-features = [\"panic_immediate_abort\"]" >> .cargo/config.toml \
    && rustup component add rust-src

ARG ARCH=x86_64

RUN rustup target add ${ARCH}-unknown-linux-musl

ENV LEPTOS_OUTPUT_NAME="mdoj"
ENV LEPTOS_BIN_TARGET_TRIPLE=${ARCH}-unknown-linux-musl

RUN cargo leptos build -p frontend --release --precompress -vv

FROM scratch
WORKDIR /config
WORKDIR /

ARG ARCH=x86_64

COPY --from=builder /compiler/target/${ARCH}-unknown-linux-musl/release/frontend .
COPY --from=builder /compiler/target/site /site

ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT=site

EXPOSE 8080

CMD ["/frontend"]
