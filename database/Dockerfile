FROM rust:1.60.0 AS builder
WORKDIR /complier

RUN USER=root apt update -y
RUN USER=root apt install git musl-tools -y

RUN git clone https://github.com/readysettech/readyset.git
RUN cd readyset

RUN rustup target add ${targets}

RUN cargo install --target ${targets} --path .

FROM alpine
WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/readyset .
COPY ./config ./config
RUN mkdir log
CMD ["./backend"]