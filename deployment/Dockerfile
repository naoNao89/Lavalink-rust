# Runtime stage for pre-built binary
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libopus0 \
    libavcodec59 \
    libavformat59 \
    libavutil57 \
    libavfilter8 \
    libavdevice59 \
    python3 \
    python3-pip \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install yt-dlp for audio source extraction
RUN pip3 install --no-cache-dir yt-dlp

# Create non-root user
RUN groupadd -g 322 lavalink && \
    useradd -r -u 322 -g lavalink lavalink

# Set working directory
WORKDIR /app

# Copy pre-built binary from CI
COPY lavalink-rust /app/lavalink-rust

# Create necessary directories
RUN mkdir -p /app/logs /app/plugins && \
    chown -R lavalink:lavalink /app

# Switch to non-root user
USER lavalink

# Expose ports
EXPOSE 2333 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:2333/v4/info || exit 1

# Default command
ENTRYPOINT ["/app/lavalink-rust"]
CMD ["--config", "/app/application.yml"]
