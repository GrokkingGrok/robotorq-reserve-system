package config

import (
	"os"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// ─────────────────────────────────────────────────────────────
// Default Configuration Tests
// ─────────────────────────────────────────────────────────────

func TestLoad_Defaults(t *testing.T) {
	// Clear all environment variables
	clearEnv(t)

	cfg, err := Load()
	require.NoError(t, err)

	// Verify all defaults
	assert.Equal(t, "8080", cfg.HTTPPort)
	assert.Equal(t, "nats://nats:4222", cfg.NatsURL)
	assert.Equal(t, 1000, cfg.BatchSize)
	assert.Equal(t, 5*time.Second, cfg.FlushInterval)
	assert.Equal(t, 100000, cfg.BufferCapacity)
	assert.Equal(t, 3, cfg.NatsRetries)
	assert.Equal(t, 100*time.Millisecond, cfg.NatsBackoffBase)
	assert.Equal(t, "info", cfg.LogLevel)
}

func TestMustLoad_Success(t *testing.T) {
	clearEnv(t)

	// Should not panic with valid defaults
	cfg := MustLoad()
	assert.NotNil(t, cfg)
	assert.Equal(t, "8080", cfg.HTTPPort)
}

func TestMustLoad_Panic(t *testing.T) {
	clearEnv(t)
	os.Setenv("BATCH_SIZE", "invalid")
	defer os.Unsetenv("BATCH_SIZE")

	// Should panic with invalid config
	assert.Panics(t, func() {
		MustLoad()
	})
}

// ─────────────────────────────────────────────────────────────
// Environment Variable Parsing Tests
// ─────────────────────────────────────────────────────────────

func TestLoad_HTTPPort(t *testing.T) {
	clearEnv(t)
	os.Setenv("HTTP_PORT", "9090")
	defer os.Unsetenv("HTTP_PORT")

	cfg, err := Load()
	require.NoError(t, err)
	assert.Equal(t, "9090", cfg.HTTPPort)
}

func TestLoad_NatsURL(t *testing.T) {
	clearEnv(t)
	os.Setenv("NATS_URL", "nats://localhost:4222")
	defer os.Unsetenv("NATS_URL")

	cfg, err := Load()
	require.NoError(t, err)
	assert.Equal(t, "nats://localhost:4222", cfg.NatsURL)
}

func TestLoad_BatchSize(t *testing.T) {
	clearEnv(t)
	os.Setenv("BATCH_SIZE", "500")
	defer os.Unsetenv("BATCH_SIZE")

	cfg, err := Load()
	require.NoError(t, err)
	assert.Equal(t, 500, cfg.BatchSize)
}

func TestLoad_BatchSize_Invalid(t *testing.T) {
	clearEnv(t)
	os.Setenv("BATCH_SIZE", "not-a-number")
	defer os.Unsetenv("BATCH_SIZE")

	_, err := Load()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid BATCH_SIZE")
}

func TestLoad_FlushInterval(t *testing.T) {
	clearEnv(t)
	os.Setenv("FLUSH_INTERVAL", "10s")
	defer os.Unsetenv("FLUSH_INTERVAL")

	cfg, err := Load()
	require.NoError(t, err)
	assert.Equal(t, 10*time.Second, cfg.FlushInterval)
}

func TestLoad_FlushInterval_Invalid(t *testing.T) {
	clearEnv(t)
	os.Setenv("FLUSH_INTERVAL", "invalid-duration")
	defer os.Unsetenv("FLUSH_INTERVAL")

	_, err := Load()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid FLUSH_INTERVAL")
}

func TestLoad_BufferCapacity(t *testing.T) {
	clearEnv(t)
	os.Setenv("BUFFER_CAPACITY", "50000")
	defer os.Unsetenv("BUFFER_CAPACITY")

	cfg, err := Load()
	require.NoError(t, err)
	assert.Equal(t, 50000, cfg.BufferCapacity)
}

func TestLoad_BufferCapacity_Invalid(t *testing.T) {
	clearEnv(t)
	os.Setenv("BUFFER_CAPACITY", "abc")
	defer os.Unsetenv("BUFFER_CAPACITY")

	_, err := Load()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid BUFFER_CAPACITY")
}

func TestLoad_NatsRetries(t *testing.T) {
	clearEnv(t)
	os.Setenv("NATS_RETRIES", "5")
	defer os.Unsetenv("NATS_RETRIES")

	cfg, err := Load()
	require.NoError(t, err)
	assert.Equal(t, 5, cfg.NatsRetries)
}

func TestLoad_NatsRetries_Invalid(t *testing.T) {
	clearEnv(t)
	os.Setenv("NATS_RETRIES", "xyz")
	defer os.Unsetenv("NATS_RETRIES")

	_, err := Load()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid NATS_RETRIES")
}

func TestLoad_NatsBackoffBase(t *testing.T) {
	clearEnv(t)
	os.Setenv("NATS_BACKOFF_BASE", "200ms")
	defer os.Unsetenv("NATS_BACKOFF_BASE")

	cfg, err := Load()
	require.NoError(t, err)
	assert.Equal(t, 200*time.Millisecond, cfg.NatsBackoffBase)
}

func TestLoad_NatsBackoffBase_Invalid(t *testing.T) {
	clearEnv(t)
	os.Setenv("NATS_BACKOFF_BASE", "not-a-duration")
	defer os.Unsetenv("NATS_BACKOFF_BASE")

	_, err := Load()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid NATS_BACKOFF_BASE")
}

func TestLoad_LogLevel(t *testing.T) {
	clearEnv(t)
	os.Setenv("LOG_LEVEL", "debug")
	defer os.Unsetenv("LOG_LEVEL")

	cfg, err := Load()
	require.NoError(t, err)
	assert.Equal(t, "debug", cfg.LogLevel)
}

// ─────────────────────────────────────────────────────────────
// Validation Tests
// ─────────────────────────────────────────────────────────────

func TestValidate_EmptyHTTPPort(t *testing.T) {
	cfg := &Config{HTTPPort: ""}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "HTTP_PORT cannot be empty")
}

func TestValidate_EmptyNatsURL(t *testing.T) {
	cfg := &Config{
		HTTPPort: "8080",
		NatsURL:  "",
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "NATS_URL cannot be empty")
}

func TestValidate_BatchSizeTooSmall(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       0,
		FlushInterval:   5 * time.Second,
		BufferCapacity:  100000,
		NatsRetries:     3,
		NatsBackoffBase: 100 * time.Millisecond,
		LogLevel:        "info",
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "BATCH_SIZE must be at least 1")
}

func TestValidate_BatchSizeTooLarge(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       200000,
		FlushInterval:   5 * time.Second,
		BufferCapacity:  100000,
		NatsRetries:     3,
		NatsBackoffBase: 100 * time.Millisecond,
		LogLevel:        "info",
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "BATCH_SIZE must not exceed 100000")
}

func TestValidate_FlushIntervalTooSmall(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       1000,
		FlushInterval:   50 * time.Millisecond,
		BufferCapacity:  100000,
		NatsRetries:     3,
		NatsBackoffBase: 100 * time.Millisecond,
		LogLevel:        "info",
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "FLUSH_INTERVAL must be at least 100ms")
}

func TestValidate_FlushIntervalTooLarge(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       1000,
		FlushInterval:   2 * time.Hour,
		BufferCapacity:  100000,
		NatsRetries:     3,
		NatsBackoffBase: 100 * time.Millisecond,
		LogLevel:        "info",
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "FLUSH_INTERVAL must not exceed 1 hour")
}

func TestValidate_BufferCapacityTooSmall(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       1000,
		FlushInterval:   5 * time.Second,
		BufferCapacity:  50,
		NatsRetries:     3,
		NatsBackoffBase: 100 * time.Millisecond,
		LogLevel:        "info",
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "BUFFER_CAPACITY must be at least 100")
}

func TestValidate_BufferCapacityTooLarge(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       1000,
		FlushInterval:   5 * time.Second,
		BufferCapacity:  20000000,
		NatsRetries:     3,
		NatsBackoffBase: 100 * time.Millisecond,
		LogLevel:        "info",
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "BUFFER_CAPACITY must not exceed 10 million")
}

func TestValidate_NatsRetriesNegative(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       1000,
		FlushInterval:   5 * time.Second,
		BufferCapacity:  100000,
		NatsRetries:     -1,
		NatsBackoffBase: 100 * time.Millisecond,
		LogLevel:        "info",
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "NATS_RETRIES must be non-negative")
}

func TestValidate_NatsRetriesTooLarge(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       1000,
		FlushInterval:   5 * time.Second,
		BufferCapacity:  100000,
		NatsRetries:     15,
		NatsBackoffBase: 100 * time.Millisecond,
		LogLevel:        "info",
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "NATS_RETRIES must not exceed 10")
}

func TestValidate_NatsBackoffTooSmall(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       1000,
		FlushInterval:   5 * time.Second,
		BufferCapacity:  100000,
		NatsRetries:     3,
		NatsBackoffBase: 5 * time.Millisecond,
		LogLevel:        "info",
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "NATS_BACKOFF_BASE must be at least 10ms")
}

func TestValidate_NatsBackoffTooLarge(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       1000,
		FlushInterval:   5 * time.Second,
		BufferCapacity:  100000,
		NatsRetries:     3,
		NatsBackoffBase: 15 * time.Second,
		LogLevel:        "info",
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "NATS_BACKOFF_BASE must not exceed 10 seconds")
}

func TestValidate_InvalidLogLevel(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       1000,
		FlushInterval:   5 * time.Second,
		BufferCapacity:  100000,
		NatsRetries:     3,
		NatsBackoffBase: 100 * time.Millisecond,
		LogLevel:        "trace", // Invalid
	}
	err := cfg.Validate()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "LOG_LEVEL must be one of: debug, info, warn, error")
}

func TestValidate_AllLogLevels(t *testing.T) {
	validLevels := []string{"debug", "info", "warn", "error"}

	for _, level := range validLevels {
		cfg := &Config{
			HTTPPort:        "8080",
			NatsURL:         "nats://nats:4222",
			BatchSize:       1000,
			FlushInterval:   5 * time.Second,
			BufferCapacity:  100000,
			NatsRetries:     3,
			NatsBackoffBase: 100 * time.Millisecond,
			LogLevel:        level,
		}
		err := cfg.Validate()
		assert.NoError(t, err, "Log level %s should be valid", level)
	}
}

// ─────────────────────────────────────────────────────────────
// String Representation Tests
// ─────────────────────────────────────────────────────────────

func TestConfig_String(t *testing.T) {
	cfg := &Config{
		HTTPPort:        "8080",
		NatsURL:         "nats://nats:4222",
		BatchSize:       1000,
		FlushInterval:   5 * time.Second,
		BufferCapacity:  100000,
		NatsRetries:     3,
		NatsBackoffBase: 100 * time.Millisecond,
		LogLevel:        "info",
	}

	str := cfg.String()

	// Verify all fields are present in output
	assert.Contains(t, str, "8080")
	assert.Contains(t, str, "nats://nats:4222")
	assert.Contains(t, str, "1000")
	assert.Contains(t, str, "5s")
	assert.Contains(t, str, "100000")
	assert.Contains(t, str, "3")
	assert.Contains(t, str, "100ms")
	assert.Contains(t, str, "info")
}

// ─────────────────────────────────────────────────────────────
// Integration Tests
// ─────────────────────────────────────────────────────────────

func TestLoad_AllEnvironmentVariables(t *testing.T) {
	clearEnv(t)

	// Set all environment variables
	os.Setenv("HTTP_PORT", "9090")
	os.Setenv("NATS_URL", "nats://localhost:4222")
	os.Setenv("BATCH_SIZE", "2000")
	os.Setenv("FLUSH_INTERVAL", "10s")
	os.Setenv("BUFFER_CAPACITY", "200000")
	os.Setenv("NATS_RETRIES", "5")
	os.Setenv("NATS_BACKOFF_BASE", "200ms")
	os.Setenv("LOG_LEVEL", "debug")

	defer func() {
		os.Unsetenv("HTTP_PORT")
		os.Unsetenv("NATS_URL")
		os.Unsetenv("BATCH_SIZE")
		os.Unsetenv("FLUSH_INTERVAL")
		os.Unsetenv("BUFFER_CAPACITY")
		os.Unsetenv("NATS_RETRIES")
		os.Unsetenv("NATS_BACKOFF_BASE")
		os.Unsetenv("LOG_LEVEL")
	}()

	cfg, err := Load()
	require.NoError(t, err)

	assert.Equal(t, "9090", cfg.HTTPPort)
	assert.Equal(t, "nats://localhost:4222", cfg.NatsURL)
	assert.Equal(t, 2000, cfg.BatchSize)
	assert.Equal(t, 10*time.Second, cfg.FlushInterval)
	assert.Equal(t, 200000, cfg.BufferCapacity)
	assert.Equal(t, 5, cfg.NatsRetries)
	assert.Equal(t, 200*time.Millisecond, cfg.NatsBackoffBase)
	assert.Equal(t, "debug", cfg.LogLevel)
}

// ─────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────

// clearEnv clears all Mint-related environment variables for clean testing.
func clearEnv(t *testing.T) {
	vars := []string{
		"HTTP_PORT",
		"NATS_URL",
		"BATCH_SIZE",
		"FLUSH_INTERVAL",
		"BUFFER_CAPACITY",
		"NATS_RETRIES",
		"NATS_BACKOFF_BASE",
		"LOG_LEVEL",
	}

	for _, v := range vars {
		os.Unsetenv(v)
	}
}
