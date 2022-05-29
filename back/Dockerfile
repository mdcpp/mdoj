FROM rust:1.60.0 AS builder
WORKDIR /complier

# install musl-gcc for ring
RUN USER=root apt update -y
RUN USER=root apt install musl-tools -y

RUN rustup target add ${targets}

RUN USER=root cargo new mdoj
WORKDIR /complier/mdoj
COPY . .

RUN cargo install --target ${targets} --path .

FROM alpine
WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/mdoj .
COPY ./config ./config
RUN mkdir log
CMD ["./mdoj"]