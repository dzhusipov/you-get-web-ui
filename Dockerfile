FROM rust:1.82 AS builder

WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

# Install Python, pip, and git
RUN apt-get update && \
    apt-get install -y python3-pip git && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Clone the you-get repository
RUN git clone https://github.com/soimort/you-get /usr/src/you-get

RUN pip install dukpy --break-system-packages

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/you-get-web-ui .
COPY static/ static/
COPY config/ config/
COPY downloads/ downloads/

VOLUME ["/app/downloads"]
RUN ln -s /app/downloads /downloads
RUN ln -s /usr/src/you-get /app/you-get
EXPOSE 8080
CMD ["./you-get-web-ui"]