# Client build stage
FROM node:24-slim AS client-builder

WORKDIR /usr/src/app

# Copy manifest
COPY package.json .
COPY package-lock.json .

# Install dependencies
RUN npm install

# Copy client source
COPY client ./client

# Build the client
RUN npm run build

# Server build stage
FROM rust:1.93-slim AS server-builder

WORKDIR /usr/src/app

# Copy manifests
COPY Cargo.toml ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy server source
COPY src ./src

# Copy client files
COPY --from=client-builder /usr/src/app/client/dist ./client/dist

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM gcr.io/distroless/cc-debian13:latest

WORKDIR /app

# Copy binary from server builder
COPY --from=server-builder /usr/src/app/target/release/log-bin /app/log-bin

# Copy client files
COPY --from=client-builder /usr/src/app/client/dist ./client/dist

# Set environment
ENV PORT=8080
ENV RUST_LOG=info

EXPOSE 8080

CMD ["/app/log-bin"]
