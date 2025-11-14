package config

import (
	"os"
	"strconv"
	"time"
)

// Config holds all configuration for the Refinery service
type Config struct {
	// NATS connection URL
	NatsURL string

	// Ingot batching interval (how often to send batches to Mint)
	IngotBatchInterval time.Duration

	// Maximum retry attempts for Mint publishing
	MintMaxRetries int

	// Base delay for exponential backoff (doubles each retry: 1s, 2s, 4s)
	MintBaseDelay time.Duration

	// Queue sizes
	JouleQueueSize int
	RoboQueueSize  int
}

// LoadConfig reads configuration from environment variables with sensible defaults
func LoadConfig() *Config {
	return &Config{
		NatsURL:            getEnv("NATS_URL", "nats://localhost:4222"),
		IngotBatchInterval: getDurationEnv("REFINERY_INGOT_BATCH_INTERVAL", 60*time.Second),
		MintMaxRetries:     getIntEnv("MINT_MAX_RETRIES", 3),
		MintBaseDelay:      getDurationEnv("MINT_BASE_DELAY", 1*time.Second),
		JouleQueueSize:     getIntEnv("REFINERY_JOULE_QUEUE_SIZE", 1000),
		RoboQueueSize:      getIntEnv("REFINERY_ROBO_QUEUE_SIZE", 1000),
	}
}

// Helper functions for reading environment variables with defaults

func getEnv(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

func getIntEnv(key string, defaultValue int) int {
	if value := os.Getenv(key); value != "" {
		if intVal, err := strconv.Atoi(value); err == nil {
			return intVal
		}
	}
	return defaultValue
}

func getDurationEnv(key string, defaultValue time.Duration) time.Duration {
	if value := os.Getenv(key); value != "" {
		// Try parsing as seconds first
		if seconds, err := strconv.Atoi(value); err == nil {
			return time.Duration(seconds) * time.Second
		}
		// Try parsing as duration string (e.g., "60s", "1m")
		if duration, err := time.ParseDuration(value); err == nil {
			return duration
		}
	}
	return defaultValue
}
