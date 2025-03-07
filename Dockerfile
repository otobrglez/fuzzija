FROM rust:1.85 AS builder
ENV RUST_LOG=info
ENV INDEXES_FOLDER=/indexes
ENV STORAGE_FOLDER=/tmp

WORKDIR /usr/src/app
COPY . .

RUN mkdir -p /indexes /tmp && \
    cargo build --release && \
    ./target/release/fuzzija --reindex

FROM debian:bookworm-slim

RUN apt-get update -yy && \
    apt-get install -yy openssl ca-certificates bash &&  \
    rm -rf /var/lib/apt/lists/*

ENV INDEXES_FOLDER=/indexes
ENV STORAGE_FOLDER=/tmp
ENV RUST_LOG=info

RUN mkdir -p /indexes /tmp
COPY --from=builder /usr/src/app/indexes /indexes

COPY --from=builder /usr/src/app/target/release/fuzzija-server \
    /usr/local/bin/fuzzija-server

COPY --from=builder /usr/src/app/target/release/fuzzija \
    /usr/local/bin/fuzzija

CMD ["fuzzija-server"]
