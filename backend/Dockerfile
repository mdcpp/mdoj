FROM rust:1.60.0 AS builder
WORKDIR /complier

# install musl-gcc for ring
RUN USER=root apt update -y
# RUN USER=root apt install musl-tools -y

RUN rustup target add ${targets}

RUN USER=root cargo new backend
WORKDIR /complier/backend
COPY . .

RUN cargo install --target ${targets} --path .

FROM alpine
WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/backend .
COPY ./config ./config
RUN mkdir log
CMD ["./backend"]