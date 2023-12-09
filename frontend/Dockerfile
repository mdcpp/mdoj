From nginx:1.25.3-alpine-slim

WORKDIR /grpc_cert
COPY ./*.pem .
WORKDIR /
COPY index.conf /etc/nginx/conf.d/default.conf
COPY dist/* /var/www/html/

