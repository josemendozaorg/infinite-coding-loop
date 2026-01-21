# Dockerfile for Infinite Coding Loop TUI
# Uses multi-stage build to keep image size small

# --- Builder Stage ---
FROM rust:bookworm as builder

WORKDIR /usr/src/app

# Copy source code
COPY . .

# Build the application in release mode
# targeting the 'tui' binary
RUN cargo build --release --bin tui

# --- Runtime Stage ---
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    sqlite3 \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Download ttyd static binary
ADD https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.x86_64 /usr/local/bin/ttyd
RUN chmod +x /usr/local/bin/ttyd

# Copy the compiled binary from builder
COPY --from=builder /usr/src/app/target/release/tui ./infinite-coding-loop

# Copy the marketplace directory for default content
COPY --from=builder /usr/src/app/marketplace ./marketplace

# Expose the ttyd port
EXPOSE 8080

# clear screen and ensure terminal capabilities
ENV TERM=xterm-256color

# Run ttyd serving the application
# -W: Allow writing (input)
# -p 8080: Port
ENTRYPOINT ["ttyd", "-W", "-p", "8080", "./infinite-coding-loop"]
