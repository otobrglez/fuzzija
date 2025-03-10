FROM rust:1.85 AS builder
ENV RUST_LOG=info
ENV INDEXES_FOLDER=/usr/src/app/indexes
ENV STORAGE_FOLDER=/usr/src/app/raw-data

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release && \
    ./target/release/fuzzija --reindex

FROM debian:bookworm-slim

ENV INDEXES_FOLDER=/usr/src/app/indexes
ENV STORAGE_FOLDER=/usr/src/app/raw-data
ENV RUST_LOG=info

RUN apt-get update -yy && \
    apt-get install -yy openssl ca-certificates bash &&  \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/indexes /usr/src/app/indexes

COPY --from=builder /usr/src/app/target/release/fuzzija-server \
    /usr/local/bin/fuzzija-server

COPY --from=builder /usr/src/app/target/release/fuzzija \
    /usr/local/bin/fuzzija

CMD ["fuzzija-server"]
