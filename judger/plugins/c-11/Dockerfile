FROM alpine:3.14

RUN apk add build-base

WORKDIR /
COPY spec.toml /

ENTRYPOINT [ "/usr/bin/cc" ]