// Package config provides configuration loading and validation for the Mint service.
package config

import (
	"fmt"
	"os"
	"strconv"
	"time"
)

// Config holds all Mint service configuration.
// Values are loaded from environment variables with sensible defaults.
type Config struct {
	// HTTP server
	HTTPPort string // Default: "8080"

	// NATS connection
	NatsURL string // Default: "nats://nats:4222"

	// Batch processing
	BatchSize     int           // Default: 1000 ingots
	FlushInterval time.Duration // Default: 5s

	// Buffering
	BufferCapacity int // Default: 100000 ingots

	// Retry configuration
	NatsRetries     int           // Default: 3
	NatsBackoffBase time.Duration // Default: 100ms

	// Logging
	LogLevel string // Default: "info"
}

// Load reads configuration from environment variables with defaults.
//
// Environment Variables:
//   - HTTP_PORT: HTTP server port (default: "8080")
//   - NATS_URL: NATS server URL (default: "nats://nats:4222")
//   - BATCH_SIZE: Number of ingots to accumulate before processing (default: 1000)
//   - FLUSH_INTERVAL: Interval for partial batch flushing (default: "5s")
//   - BUFFER_CAPACITY: Maximum ingot buffer size (default: 100000)
//   - NATS_RETRIES: Number of NATS publish retries (default: 3)
//   - NATS_BACKOFF_BASE: Base duration for exponential backoff (default: "100ms")
//   - LOG_LEVEL: Logging level: debug, info, warn, error (default: "info")
//
// Returns error if any configuration value is invalid.
func Load() (*Config, error) {
	cfg := &Config{
		HTTPPort:        getEnv("HTTP_PORT", "8080"),
		NatsURL:         getEnv("NATS_URL", "nats://nats:4222"),
		LogLevel:        getEnv("LOG_LEVEL", "info"),
		BatchSize:       1000,
		FlushInterval:   5 * time.Second,
		BufferCapacity:  100000,
		NatsRetries:     3,
		NatsBackoffBase: 100 * time.Millisecond,
	}

	// Parse BATCH_SIZE
	if val := os.Getenv("BATCH_SIZE"); val != "" {
		size, err := strconv.Atoi(val)
		if err != nil {
			return nil, fmt.Errorf("invalid BATCH_SIZE: %w", err)
		}
		cfg.BatchSize = size
	}

	// Parse FLUSH_INTERVAL
	if val := os.Getenv("FLUSH_INTERVAL"); val != "" {
		interval, err := time.ParseDuration(val)
		if err != nil {
			return nil, fmt.Errorf("invalid FLUSH_INTERVAL: %w", err)
		}
		cfg.FlushInterval = interval
	}

	// Parse BUFFER_CAPACITY
	if val := os.Getenv("BUFFER_CAPACITY"); val != "" {
		capacity, err := strconv.Atoi(val)
		if err != nil {
			return nil, fmt.Errorf("invalid BUFFER_CAPACITY: %w", err)
		}
		cfg.BufferCapacity = capacity
	}

	// Parse NATS_RETRIES
	if val := os.Getenv("NATS_RETRIES"); val != "" {
		retries, err := strconv.Atoi(val)
		if err != nil {
			return nil, fmt.Errorf("invalid NATS_RETRIES: %w", err)
		}
		cfg.NatsRetries = retries
	}

	// Parse NATS_BACKOFF_BASE
	if val := os.Getenv("NATS_BACKOFF_BASE"); val != "" {
		backoff, err := time.ParseDuration(val)
		if err != nil {
			return nil, fmt.Errorf("invalid NATS_BACKOFF_BASE: %w", err)
		}
		cfg.NatsBackoffBase = backoff
	}

	// Validate configuration
	if err := cfg.Validate(); err != nil {
		return nil, fmt.Errorf("configuration validation failed: %w", err)
	}

	return cfg, nil
}

// Validate ensures all configuration values are within acceptable ranges.
func (c *Config) Validate() error {
	// HTTP port validation
	if c.HTTPPort == "" {
		return fmt.Errorf("HTTP_PORT cannot be empty")
	}

	// NATS URL validation
	if c.NatsURL == "" {
		return fmt.Errorf("NATS_URL cannot be empty")
	}

	// Batch size validation
	if c.BatchSize < 1 {
		return fmt.Errorf("BATCH_SIZE must be at least 1, got %d", c.BatchSize)
	}
	if c.BatchSize > 100000 {
		return fmt.Errorf("BATCH_SIZE must not exceed 100000, got %d", c.BatchSize)
	}

	// Flush interval validation
	if c.FlushInterval < 100*time.Millisecond {
		return fmt.Errorf("FLUSH_INTERVAL must be at least 100ms, got %v", c.FlushInterval)
	}
	if c.FlushInterval > 1*time.Hour {
		return fmt.Errorf("FLUSH_INTERVAL must not exceed 1 hour, got %v", c.FlushInterval)
	}

	// Buffer capacity validation
	if c.BufferCapacity < 100 {
		return fmt.Errorf("BUFFER_CAPACITY must be at least 100, got %d", c.BufferCapacity)
	}
	if c.BufferCapacity > 10000000 {
		return fmt.Errorf("BUFFER_CAPACITY must not exceed 10 million, got %d", c.BufferCapacity)
	}

	// NATS retries validation
	if c.NatsRetries < 0 {
		return fmt.Errorf("NATS_RETRIES must be non-negative, got %d", c.NatsRetries)
	}
	if c.NatsRetries > 10 {
		return fmt.Errorf("NATS_RETRIES must not exceed 10, got %d", c.NatsRetries)
	}

	// NATS backoff validation
	if c.NatsBackoffBase < 10*time.Millisecond {
		return fmt.Errorf("NATS_BACKOFF_BASE must be at least 10ms, got %v", c.NatsBackoffBase)
	}
	if c.NatsBackoffBase > 10*time.Second {
		return fmt.Errorf("NATS_BACKOFF_BASE must not exceed 10 seconds, got %v", c.NatsBackoffBase)
	}

	// Log level validation
	validLogLevels := map[string]bool{
		"debug": true,
		"info":  true,
		"warn":  true,
		"error": true,
	}
	if !validLogLevels[c.LogLevel] {
		return fmt.Errorf("LOG_LEVEL must be one of: debug, info, warn, error; got %q", c.LogLevel)
	}

	return nil
}

// getEnv retrieves an environment variable or returns a default value.
func getEnv(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

// MustLoad loads configuration and panics on error.
// Use this in main.go for fail-fast behavior.
func MustLoad() *Config {
	cfg, err := Load()
	if err != nil {
		panic(fmt.Sprintf("failed to load configuration: %v", err))
	}
	return cfg
}

// String returns a human-readable representation of the configuration.
// Useful for startup logging.
func (c *Config) String() string {
	return fmt.Sprintf(`Mint Configuration:
  HTTP Port:         %s
  NATS URL:          %s
  Batch Size:        %d ingots
  Flush Interval:    %v
  Buffer Capacity:   %d ingots
  NATS Retries:      %d
  NATS Backoff Base: %v
  Log Level:         %s`,
		c.HTTPPort,
		c.NatsURL,
		c.BatchSize,
		c.FlushInterval,
		c.BufferCapacity,
		c.NatsRetries,
		c.NatsBackoffBase,
		c.LogLevel,
	)
}
