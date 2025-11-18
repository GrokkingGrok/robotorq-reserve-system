// Package config provides configuration loading and validation for the DistoDam service.
package config

import (
	"fmt"
	"os"
	"strconv"
	"time"
)

// LoanPolicy defines how DistoDam handles loans from DistoVault to StakeVault.
type LoanPolicy string

const (
	// LoanPolicyNatural - Natural equilibrium (no Torq bump, default)
	LoanPolicyNatural LoanPolicy = "natural"
	// LoanPolicyTorqBumpImmediate - Torq bump on every loan
	LoanPolicyTorqBumpImmediate LoanPolicy = "torq_bump_immediate"
	// LoanPolicyTorqBumpThreshold - Torq bump when loan/DistoVault ratio exceeds threshold
	LoanPolicyTorqBumpThreshold LoanPolicy = "torq_bump_threshold"
	// LoanPolicyTorqBumpDelayed - Torq bump after delay period
	LoanPolicyTorqBumpDelayed LoanPolicy = "torq_bump_delayed"
)

// Config holds all DistoDam service configuration.
// Values are loaded from environment variables with sensible defaults.
type Config struct {
	// HTTP server
	HTTPPort string // Default: "8082"

	// NATS connection
	NatsURL string // Default: "nats://nats:4222"

	// NATS topics
	MintBatchesTopic       string // Default: "mint.batches"
	ContractsApprovedTopic string // Default: "contracts.approved"
	ContractsFundedTopic   string // Default: "contracts.funded"
	UBDRequestsTopic       string // Default: "ubd.requests"
	UBDFundedTopic         string // Default: "ubd.funded"

	// Vault genesis bootstrap (Phase 6 - internal state initialization)
	InitialStakeVaultRT         float64 // Default: 0.0
	InitialDistoVaultRT         float64 // Default: 0.0
	GenesisBootstrapStakePct    float64 // Default: 0.95 (95%)
	GenesisBootstrapDistoPct    float64 // Default: 0.05 (5%)

	// Loan policy
	LoanPolicy LoanPolicy // Default: "natural"

	// Torq bump emergency lever (natural equilibrium is default)
	TorqBumpEnabled        bool    // Default: false
	TorqBumpThresholdRatio float64 // Default: 0.10 (StakeVault < 10% of DistoVault)
	TorqBumpAmountRT       float64 // Default: 0.05 RT
	TorqBumpDelayHours     int     // Default: 24 hours
	TorqBumpThresholdPct   float64 // Default: 0.30 (loan/DistoVault ratio)

	// Vault water marks (future - multi-dam rebalancing)
	StakeLowWaterMarkRT  float64 // Default: 0.001 RT
	StakeHighWaterMarkRT float64 // Default: 0.01 RT
	DistoLowWaterMarkRT  float64 // Default: 0.0005 RT
	DistoHighWaterMarkRT float64 // Default: 0.005 RT

	// Retry configuration
	NatsRetries     int           // Default: 3
	NatsBackoffBase time.Duration // Default: 100ms

	// Logging
	LogLevel string // Default: "info"

	// Identity
	DamID string // Default: "distodam-001"

	// Admin recovery (Phase 6 - for manual vault corrections)
	AdminBearerToken string // Required for /admin/vault/credit endpoint
}

// Load reads configuration from environment variables with defaults.
//
// Environment Variables:
//   - HTTP_PORT: HTTP server port (default: "8082")
//   - NATS_URL: NATS server URL (default: "nats://nats:4222")
//   - MINT_BATCHES_TOPIC: Topic for Mint batch events (default: "mint.batches")
//   - CONTRACTS_APPROVED_TOPIC: Topic for approved contracts (default: "contracts.approved")
//   - CONTRACTS_FUNDED_TOPIC: Topic for funded contracts (default: "contracts.funded")
//   - UBD_REQUESTS_TOPIC: Topic for UBD requests (default: "ubd.requests")
//   - UBD_FUNDED_TOPIC: Topic for funded UBD (default: "ubd.funded")
//   - INITIAL_STAKE_VAULT_RT: Initial StakeVault balance in RT (default: 0.0)
//   - INITIAL_DISTO_VAULT_RT: Initial DistoVault balance in RT (default: 0.0)
//   - GENESIS_BOOTSTRAP_STAKE_PCT: StakeVault percentage (default: 0.95)
//   - GENESIS_BOOTSTRAP_DISTO_PCT: DistoVault percentage (default: 0.05)
//   - DISTODAM_LOAN_POLICY: Loan policy (default: "natural")
//   - TORQ_BUMP_ENABLED: Enable Torq bump (default: false)
//   - TORQ_BUMP_THRESHOLD_RATIO: StakeVault/DistoVault trigger ratio (default: 0.10)
//   - TORQ_BUMP_AMOUNT_RT: Torq bump amount in RT (default: 0.05)
//   - TORQ_BUMP_DELAY_HOURS: Delay before Torq bump (default: 24)
//   - TORQ_BUMP_THRESHOLD_PCT: Loan/DistoVault ratio threshold (default: 0.30)
//   - STAKE_LOW_WATER_MARK_RT: StakeVault low water mark (default: 0.001)
//   - STAKE_HIGH_WATER_MARK_RT: StakeVault high water mark (default: 0.01)
//   - DISTO_LOW_WATER_MARK_RT: DistoVault low water mark (default: 0.0005)
//   - DISTO_HIGH_WATER_MARK_RT: DistoVault high water mark (default: 0.005)
//   - NATS_RETRIES: Number of NATS publish retries (default: 3)
//   - NATS_BACKOFF_BASE: Base duration for exponential backoff (default: "100ms")
//   - LOG_LEVEL: Logging level: debug, info, warn, error (default: "info")
//   - DAM_ID: DistoDam instance identifier (default: "distodam-001")
//   - ADMIN_BEARER_TOKEN: Bearer token for admin endpoints (required for /admin/*)
//
// Returns error if any configuration value is invalid.
func Load() (*Config, error) {
	cfg := &Config{
		HTTPPort:                 getEnv("HTTP_PORT", "8082"),
		NatsURL:                  getEnv("NATS_URL", "nats://nats:4222"),
		MintBatchesTopic:         getEnv("MINT_BATCHES_TOPIC", "mint.batches"),
		ContractsApprovedTopic:   getEnv("CONTRACTS_APPROVED_TOPIC", "contracts.approved"),
		ContractsFundedTopic:     getEnv("CONTRACTS_FUNDED_TOPIC", "contracts.funded"),
		UBDRequestsTopic:         getEnv("UBD_REQUESTS_TOPIC", "ubd.requests"),
		UBDFundedTopic:           getEnv("UBD_FUNDED_TOPIC", "ubd.funded"),
		InitialStakeVaultRT:      0.0,
		InitialDistoVaultRT:      0.0,
		GenesisBootstrapStakePct: 0.95,
		GenesisBootstrapDistoPct: 0.05,
		LoanPolicy:               LoanPolicyNatural,
		TorqBumpEnabled:          false,
		TorqBumpThresholdRatio:   0.10,
		TorqBumpAmountRT:         0.05,
		TorqBumpDelayHours:       24,
		TorqBumpThresholdPct:     0.30,
		StakeLowWaterMarkRT:      0.001,
		StakeHighWaterMarkRT:     0.01,
		DistoLowWaterMarkRT:      0.0005,
		DistoHighWaterMarkRT:     0.005,
		NatsRetries:              3,
		NatsBackoffBase:          100 * time.Millisecond,
		LogLevel:                 getEnv("LOG_LEVEL", "info"),
		DamID:                    getEnv("DAM_ID", "distodam-001"),
		AdminBearerToken:         os.Getenv("ADMIN_BEARER_TOKEN"),
	}

	// Parse INITIAL_STAKE_VAULT_RT
	if val := os.Getenv("INITIAL_STAKE_VAULT_RT"); val != "" {
		amount, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid INITIAL_STAKE_VAULT_RT: %w", err)
		}
		cfg.InitialStakeVaultRT = amount
	}

	// Parse INITIAL_DISTO_VAULT_RT
	if val := os.Getenv("INITIAL_DISTO_VAULT_RT"); val != "" {
		amount, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid INITIAL_DISTO_VAULT_RT: %w", err)
		}
		cfg.InitialDistoVaultRT = amount
	}

	// Parse GENESIS_BOOTSTRAP_STAKE_PCT
	if val := os.Getenv("GENESIS_BOOTSTRAP_STAKE_PCT"); val != "" {
		pct, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid GENESIS_BOOTSTRAP_STAKE_PCT: %w", err)
		}
		cfg.GenesisBootstrapStakePct = pct
	}

	// Parse GENESIS_BOOTSTRAP_DISTO_PCT
	if val := os.Getenv("GENESIS_BOOTSTRAP_DISTO_PCT"); val != "" {
		pct, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid GENESIS_BOOTSTRAP_DISTO_PCT: %w", err)
		}
		cfg.GenesisBootstrapDistoPct = pct
	}

	// Parse DISTODAM_LOAN_POLICY
	if val := os.Getenv("DISTODAM_LOAN_POLICY"); val != "" {
		cfg.LoanPolicy = LoanPolicy(val)
	}

	// Parse TORQ_BUMP_ENABLED
	if val := os.Getenv("TORQ_BUMP_ENABLED"); val != "" {
		enabled, err := strconv.ParseBool(val)
		if err != nil {
			return nil, fmt.Errorf("invalid TORQ_BUMP_ENABLED: %w", err)
		}
		cfg.TorqBumpEnabled = enabled
	}

	// Parse TORQ_BUMP_THRESHOLD_RATIO
	if val := os.Getenv("TORQ_BUMP_THRESHOLD_RATIO"); val != "" {
		ratio, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid TORQ_BUMP_THRESHOLD_RATIO: %w", err)
		}
		cfg.TorqBumpThresholdRatio = ratio
	}

	// Parse TORQ_BUMP_AMOUNT_RT
	if val := os.Getenv("TORQ_BUMP_AMOUNT_RT"); val != "" {
		amount, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid TORQ_BUMP_AMOUNT_RT: %w", err)
		}
		cfg.TorqBumpAmountRT = amount
	}

	// Parse TORQ_BUMP_DELAY_HOURS
	if val := os.Getenv("TORQ_BUMP_DELAY_HOURS"); val != "" {
		hours, err := strconv.Atoi(val)
		if err != nil {
			return nil, fmt.Errorf("invalid TORQ_BUMP_DELAY_HOURS: %w", err)
		}
		cfg.TorqBumpDelayHours = hours
	}

	// Parse TORQ_BUMP_THRESHOLD_PCT
	if val := os.Getenv("TORQ_BUMP_THRESHOLD_PCT"); val != "" {
		pct, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid TORQ_BUMP_THRESHOLD_PCT: %w", err)
		}
		cfg.TorqBumpThresholdPct = pct
	}

	// Parse water marks
	if val := os.Getenv("STAKE_LOW_WATER_MARK_RT"); val != "" {
		mark, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid STAKE_LOW_WATER_MARK_RT: %w", err)
		}
		cfg.StakeLowWaterMarkRT = mark
	}

	if val := os.Getenv("STAKE_HIGH_WATER_MARK_RT"); val != "" {
		mark, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid STAKE_HIGH_WATER_MARK_RT: %w", err)
		}
		cfg.StakeHighWaterMarkRT = mark
	}

	if val := os.Getenv("DISTO_LOW_WATER_MARK_RT"); val != "" {
		mark, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid DISTO_LOW_WATER_MARK_RT: %w", err)
		}
		cfg.DistoLowWaterMarkRT = mark
	}

	if val := os.Getenv("DISTO_HIGH_WATER_MARK_RT"); val != "" {
		mark, err := strconv.ParseFloat(val, 64)
		if err != nil {
			return nil, fmt.Errorf("invalid DISTO_HIGH_WATER_MARK_RT: %w", err)
		}
		cfg.DistoHighWaterMarkRT = mark
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

	// Topic validation
	if c.MintBatchesTopic == "" {
		return fmt.Errorf("MINT_BATCHES_TOPIC cannot be empty")
	}
	if c.ContractsApprovedTopic == "" {
		return fmt.Errorf("CONTRACTS_APPROVED_TOPIC cannot be empty")
	}
	if c.ContractsFundedTopic == "" {
		return fmt.Errorf("CONTRACTS_FUNDED_TOPIC cannot be empty")
	}

	// Vault balance validation
	if c.InitialStakeVaultRT < 0 {
		return fmt.Errorf("INITIAL_STAKE_VAULT_RT must be non-negative, got %f", c.InitialStakeVaultRT)
	}
	if c.InitialDistoVaultRT < 0 {
		return fmt.Errorf("INITIAL_DISTO_VAULT_RT must be non-negative, got %f", c.InitialDistoVaultRT)
	}

	// Bootstrap percentage validation
	if c.GenesisBootstrapStakePct < 0 || c.GenesisBootstrapStakePct > 1 {
		return fmt.Errorf("GENESIS_BOOTSTRAP_STAKE_PCT must be between 0 and 1, got %f", c.GenesisBootstrapStakePct)
	}
	if c.GenesisBootstrapDistoPct < 0 || c.GenesisBootstrapDistoPct > 1 {
		return fmt.Errorf("GENESIS_BOOTSTRAP_DISTO_PCT must be between 0 and 1, got %f", c.GenesisBootstrapDistoPct)
	}

	// Bootstrap percentages must sum to 1.0
	sum := c.GenesisBootstrapStakePct + c.GenesisBootstrapDistoPct
	if sum < 0.999 || sum > 1.001 { // Allow small floating point error
		return fmt.Errorf("GENESIS_BOOTSTRAP_STAKE_PCT + GENESIS_BOOTSTRAP_DISTO_PCT must equal 1.0, got %f", sum)
	}

	// Loan policy validation
	validPolicies := map[LoanPolicy]bool{
		LoanPolicyNatural:           true,
		LoanPolicyTorqBumpImmediate: true,
		LoanPolicyTorqBumpThreshold: true,
		LoanPolicyTorqBumpDelayed:   true,
	}
	if !validPolicies[c.LoanPolicy] {
		return fmt.Errorf("DISTODAM_LOAN_POLICY must be one of: natural, torq_bump_immediate, torq_bump_threshold, torq_bump_delayed; got %q", c.LoanPolicy)
	}

	// Torq bump validation
	if c.TorqBumpThresholdRatio < 0 || c.TorqBumpThresholdRatio > 1 {
		return fmt.Errorf("TORQ_BUMP_THRESHOLD_RATIO must be between 0 and 1, got %f", c.TorqBumpThresholdRatio)
	}
	if c.TorqBumpAmountRT < 0 {
		return fmt.Errorf("TORQ_BUMP_AMOUNT_RT must be non-negative, got %f", c.TorqBumpAmountRT)
	}
	if c.TorqBumpDelayHours < 12 || c.TorqBumpDelayHours > 96 {
		return fmt.Errorf("TORQ_BUMP_DELAY_HOURS must be between 12 and 96, got %d", c.TorqBumpDelayHours)
	}
	if c.TorqBumpThresholdPct < 0.10 || c.TorqBumpThresholdPct > 0.50 {
		return fmt.Errorf("TORQ_BUMP_THRESHOLD_PCT must be between 0.10 and 0.50, got %f", c.TorqBumpThresholdPct)
	}

	// Water mark validation
	if c.StakeLowWaterMarkRT < 0 {
		return fmt.Errorf("STAKE_LOW_WATER_MARK_RT must be non-negative, got %f", c.StakeLowWaterMarkRT)
	}
	if c.StakeHighWaterMarkRT < c.StakeLowWaterMarkRT {
		return fmt.Errorf("STAKE_HIGH_WATER_MARK_RT must be >= STAKE_LOW_WATER_MARK_RT, got %f < %f", c.StakeHighWaterMarkRT, c.StakeLowWaterMarkRT)
	}
	if c.DistoLowWaterMarkRT < 0 {
		return fmt.Errorf("DISTO_LOW_WATER_MARK_RT must be non-negative, got %f", c.DistoLowWaterMarkRT)
	}
	if c.DistoHighWaterMarkRT < c.DistoLowWaterMarkRT {
		return fmt.Errorf("DISTO_HIGH_WATER_MARK_RT must be >= DISTO_LOW_WATER_MARK_RT, got %f < %f", c.DistoHighWaterMarkRT, c.DistoLowWaterMarkRT)
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

	// Identity validation
	if c.DamID == "" {
		return fmt.Errorf("DAM_ID cannot be empty")
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
	return fmt.Sprintf(`DistoDam Configuration:
  HTTP Port:                  %s
  NATS URL:                   %s
  Dam ID:                     %s
  
  Topics:
    Mint Batches:             %s
    Contracts Approved:       %s
    Contracts Funded:         %s
    UBD Requests:             %s
    UBD Funded:               %s
  
  Vault Genesis Bootstrap:
    Initial StakeVault RT:    %.6f
    Initial DistoVault RT:    %.6f
    Stake Allocation:         %.1f%%
    Disto Allocation:         %.1f%%
  
  Loan Policy:
    Policy:                   %s
    Torq Bump Enabled:        %v
    Torq Bump Threshold:      %.2f (StakeVault/DistoVault ratio)
    Torq Bump Amount:         %.6f RT
    Torq Bump Delay:          %d hours
    Torq Bump Loan Threshold: %.2f (loan/DistoVault ratio)
  
  Water Marks:
    Stake Low:                %.6f RT
    Stake High:               %.6f RT
    Disto Low:                %.6f RT
    Disto High:               %.6f RT
  
  NATS:
    Retries:                  %d
    Backoff Base:             %v
  
  Logging:
    Log Level:                %s`,
		c.HTTPPort,
		c.NatsURL,
		c.DamID,
		c.MintBatchesTopic,
		c.ContractsApprovedTopic,
		c.ContractsFundedTopic,
		c.UBDRequestsTopic,
		c.UBDFundedTopic,
		c.InitialStakeVaultRT,
		c.InitialDistoVaultRT,
		c.GenesisBootstrapStakePct*100,
		c.GenesisBootstrapDistoPct*100,
		c.LoanPolicy,
		c.TorqBumpEnabled,
		c.TorqBumpThresholdRatio,
		c.TorqBumpAmountRT,
		c.TorqBumpDelayHours,
		c.TorqBumpThresholdPct,
		c.StakeLowWaterMarkRT,
		c.StakeHighWaterMarkRT,
		c.DistoLowWaterMarkRT,
		c.DistoHighWaterMarkRT,
		c.NatsRetries,
		c.NatsBackoffBase,
		c.LogLevel,
	)
}
