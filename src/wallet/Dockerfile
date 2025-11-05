# -------------------------------
# Stage 1: Build the WebAssembly package
# -------------------------------
FROM rust:1.88 AS builder
# Use the official Rust image (based on Debian) as the build environment.
# This stage compiles the Rust source code into WebAssembly (WASM) for use in the browser.

WORKDIR /app
# Set the working directory inside the container to /app.
# All subsequent commands will be executed relative to this directory.

# ----------------------------------------
# Install the required Rust and WASM tools
# ----------------------------------------
RUN rustup target add wasm32-unknown-unknown && \
    cargo install wasm-pack
# Add the WebAssembly target to Rust and install wasm-pack.
# - `wasm32-unknown-unknown` enables Rust compilation for browser-compatible WebAssembly.
# - `wasm-pack` handles building, packaging, and bundling Rust → WASM projects.

# -------------------------------
# Copy the project source files
# -------------------------------
COPY . .
# Copy the entire project directory into the container’s /app directory.
# This includes Cargo.toml, source code, and any static files required for building.

# -------------------------------
# Fetch dependencies and build
# -------------------------------
RUN cargo fetch
# Pre-fetch and cache all dependencies defined in Cargo.toml.
# Doing this first helps leverage Docker’s build cache when code changes later.

RUN wasm-pack build --target web --out-dir pkg --release
# Compile the Rust code into optimized WebAssembly output for the web target.
# - The resulting files (WASM + JS bindings) are placed in /app/pkg.
# - The `--release` flag ensures optimized, production-ready output.

# -------------------------------
# Stage 2: Build the final runtime image
# -------------------------------
FROM nginx:alpine
# Start from a lightweight Nginx base image to serve static files.
# This image contains only what’s needed to run a web server.

# -------------------------------
# Copy static site and compiled assets
# -------------------------------
COPY web /usr/share/nginx/html
# Copy your static web assets (HTML, CSS, JS, etc.) into Nginx’s default serve directory.

COPY --from=builder /app/pkg /usr/share/nginx/html/pkg
# Copy the compiled WebAssembly package from the builder stage.
# The WASM and JS loader files will be available at /usr/share/nginx/html/pkg.

COPY nginx.conf /etc/nginx/nginx.conf
# Copy a custom Nginx configuration file to override defaults (e.g., caching, routing rules).

# -------------------------------
# Configure and run Nginx
# -------------------------------
EXPOSE 3000
# Document that this container listens on port 3000.
# (Note: EXPOSE doesn’t publish the port automatically—it’s metadata for users.)

# -------------------------------
# Health check configuration
# -------------------------------
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:3000/health || exit 1
# Define a container health check to ensure the web service is responding.
# Every 30 seconds, it makes an HTTP request to /health on port 3000.
# If the request fails or times out after 3 seconds, Docker marks the container as unhealthy.
# You can implement a simple /health endpoint in your app or serve a static /health.html file.

# -------------------------------
# Default startup command
# -------------------------------
CMD ["nginx", "-g", "daemon off;"]
# Start Nginx in the foreground so Docker keeps the container running.
# The "-g 'daemon off;'" flag ensures Nginx does not fork into the background.
