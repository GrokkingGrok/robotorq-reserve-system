# Use official Go image
FROM golang:1.23

# Set working directory
WORKDIR /app

# Copy go.mod and go.sum first (for caching)
COPY go.mod go.sum ./
RUN go mod download

# Copy the rest of the code
COPY . .

# Run tests with race detector
CMD ["go", "test", "-race", "-v", "./..."]
