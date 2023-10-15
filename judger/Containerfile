FROM docker.io/libray/rust:1.71.1-buster AS builder
WORKDIR /complier

RUN USER=root apt update -y
RUN USER=root apt install musl-tools musl-gcc -y

RUN cargo install just

RUN just debian-deps

RUN just build-nsjail

RUN rustup target add ${targets}

RUN USER=root cargo new judger
WORKDIR /complier/judger
COPY . .

RUN cargo install --target ${targets} --path .

FROM scratch
WORKDIR /
RUN mkdir -p .temp
RUN mkdir -p log
RUN mkdir -p nsjail-sys/nsjail/nsjail
COPY plugins /
COPY --from=builder /usr/local/cargo/bin/judger /
COPY --from=builder nsjail/nsjail /nsjail/

RUN mkdir temp
CMD ["/judger"]