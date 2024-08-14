FROM rust:1.80.1-alpine as builder
WORKDIR /app/

RUN apk add musl-dev

COPY . .

RUN cargo install --path .

FROM scratch

COPY --from=builder /usr/local/cargo/bin/geoip2-server /geoip2-server

ENTRYPOINT ["/geoip2-server"]
