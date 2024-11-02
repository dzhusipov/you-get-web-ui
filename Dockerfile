# Stage 1: Build the Rust application
FROM --platform=linux/amd64 rust:alpine3.20 AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

# Stage 2: Create a smaller image with the built application
FROM --platform=linux/amd64 alpine:3.18 

# Install necessary dependencies
RUN apk add --no-cache python3 py3-pip git ffmpeg

# Clone the you-get repository
RUN git clone https://github.com/soimort/you-get /usr/src/you-get

RUN pip install dukpy --break-system-packages

WORKDIR /app

# Copy the built application from the builder stage
COPY --from=builder /usr/src/app/target/release/you-get-web-ui .
COPY static/ static/
COPY config/ config/
COPY downloads/ downloads/

VOLUME ["/app/downloads"]
RUN ln -s /app/downloads /downloads
RUN ln -s /usr/src/you-get /app/you-get

# Expose the necessary port
EXPOSE 8080

# Run the application
CMD ["./you-get-web-ui"]