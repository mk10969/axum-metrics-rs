##### build container #####
FROM rust:1.60.0 as builder

# RUN apt update; \
#     # https://docs.rs/openssl/0.10.32/openssl/
#     apt-get install pkg-config libssl-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo install --path .


##### run container #####
# https://github.com/GoogleContainerTools/distroless/blob/master/base/README.md
FROM gcr.io/distroless/cc

COPY --from=builder /usr/local/cargo/bin/axum-metrics-rs ./

CMD ["./axum-metrics-rs"]
