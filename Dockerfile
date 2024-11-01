FROM rust:1.70 as builder

WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && \
    apt-get install -y python3-pip && \
    pip3 install you-get && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/youtube-downloader .
COPY static/ static/

VOLUME ["/app/downloads"]
EXPOSE 8080
CMD ["./youtube-downloader"]