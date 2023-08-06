FROM rust:1.69.0-buster AS builder
WORKDIR /complier

# install musl-gcc for ring
RUN USER=root apt update -y
RUN USER=root apt install musl-tools -y

RUN cargo install just

RUN just install-deps-debian
RUN just build-nsjail

RUN rustup target add ${targets}

RUN USER=root cargo new judger
WORKDIR /complier/judger
COPY . .

RUN cargo install --target ${targets} --path .

FROM scratch
WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/judger .
COPY ./config.toml ./config.toml

RUN mkdir -p nsjail-sys/nsjail/nsjail
COPY --from=builder nsjail-sys/nsjail/nsjail nsjail-sys/nsjail/nsjail

RUN mkdir temp
CMD ["./judger"]