package config

import (
	"os"
	"testing"
	"time"
)

// TestLoad_Defaults verifies all default values are set correctly.
func TestLoad_Defaults(t *testing.T) {
	// Clear all relevant env vars
	clearEnv()

	cfg, err := Load()
	if err != nil {
		t.Fatalf("Load() failed: %v", err)
	}

	// HTTP server
	if cfg.HTTPPort != "8082" {
		t.Errorf("HTTPPort = %q, want %q", cfg.HTTPPort, "8082")
	}

	// NATS connection
	if cfg.NatsURL != "nats://nats:4222" {
		t.Errorf("NatsURL = %q, want %q", cfg.NatsURL, "nats://nats:4222")
	}

	// Topics
	if cfg.MintBatchesTopic != "mint.batches" {
		t.Errorf("MintBatchesTopic = %q, want %q", cfg.MintBatchesTopic, "mint.batches")
	}
	if cfg.ContractsApprovedTopic != "contracts.approved" {
		t.Errorf("ContractsApprovedTopic = %q, want %q", cfg.ContractsApprovedTopic, "contracts.approved")
	}
	if cfg.ContractsFundedTopic != "contracts.funded" {
		t.Errorf("ContractsFundedTopic = %q, want %q", cfg.ContractsFundedTopic, "contracts.funded")
	}

	// Vault balances
	if cfg.InitialStakeVaultRT != 0.0 {
		t.Errorf("InitialStakeVaultRT = %f, want 0.0", cfg.InitialStakeVaultRT)
	}
	if cfg.InitialDistoVaultRT != 0.0 {
		t.Errorf("InitialDistoVaultRT = %f, want 0.0", cfg.InitialDistoVaultRT)
	}

	// Bootstrap percentages
	if cfg.GenesisBootstrapStakePct != 0.95 {
		t.Errorf("GenesisBootstrapStakePct = %f, want 0.95", cfg.GenesisBootstrapStakePct)
	}
	if cfg.GenesisBootstrapDistoPct != 0.05 {
		t.Errorf("GenesisBootstrapDistoPct = %f, want 0.05", cfg.GenesisBootstrapDistoPct)
	}

	// Loan policy
	if cfg.LoanPolicy != LoanPolicyNatural {
		t.Errorf("LoanPolicy = %q, want %q", cfg.LoanPolicy, LoanPolicyNatural)
	}
	if cfg.TorqBumpEnabled != false {
		t.Errorf("TorqBumpEnabled = %v, want false", cfg.TorqBumpEnabled)
	}
	if cfg.TorqBumpThresholdRatio != 0.10 {
		t.Errorf("TorqBumpThresholdRatio = %f, want 0.10", cfg.TorqBumpThresholdRatio)
	}

	// NATS
	if cfg.NatsRetries != 3 {
		t.Errorf("NatsRetries = %d, want 3", cfg.NatsRetries)
	}
	if cfg.NatsBackoffBase != 100*time.Millisecond {
		t.Errorf("NatsBackoffBase = %v, want 100ms", cfg.NatsBackoffBase)
	}

	// Identity
	if cfg.DamID != "distodam-001" {
		t.Errorf("DamID = %q, want %q", cfg.DamID, "distodam-001")
	}

	// Log level
	if cfg.LogLevel != "info" {
		t.Errorf("LogLevel = %q, want %q", cfg.LogLevel, "info")
	}
}

// TestLoad_FromEnv verifies environment variables are parsed correctly.
func TestLoad_FromEnv(t *testing.T) {
	clearEnv()

	os.Setenv("HTTP_PORT", "9999")
	os.Setenv("NATS_URL", "nats://custom:4223")
	os.Setenv("INITIAL_STAKE_VAULT_RT", "1.5")
	os.Setenv("INITIAL_DISTO_VAULT_RT", "0.5")
	os.Setenv("GENESIS_BOOTSTRAP_STAKE_PCT", "0.80")
	os.Setenv("GENESIS_BOOTSTRAP_DISTO_PCT", "0.20")
	os.Setenv("DISTODAM_LOAN_POLICY", "torq_bump_immediate")
	os.Setenv("TORQ_BUMP_ENABLED", "true")
	os.Setenv("TORQ_BUMP_THRESHOLD_RATIO", "0.05")
	os.Setenv("TORQ_BUMP_AMOUNT_RT", "0.1")
	os.Setenv("TORQ_BUMP_DELAY_HOURS", "48")
	os.Setenv("TORQ_BUMP_THRESHOLD_PCT", "0.25")
	os.Setenv("NATS_RETRIES", "5")
	os.Setenv("NATS_BACKOFF_BASE", "200ms")
	os.Setenv("LOG_LEVEL", "debug")
	os.Setenv("DAM_ID", "distodam-test")
	defer clearEnv()

	cfg, err := Load()
	if err != nil {
		t.Fatalf("Load() failed: %v", err)
	}

	if cfg.HTTPPort != "9999" {
		t.Errorf("HTTPPort = %q, want %q", cfg.HTTPPort, "9999")
	}
	if cfg.NatsURL != "nats://custom:4223" {
		t.Errorf("NatsURL = %q, want %q", cfg.NatsURL, "nats://custom:4223")
	}
	if cfg.InitialStakeVaultRT != 1.5 {
		t.Errorf("InitialStakeVaultRT = %f, want 1.5", cfg.InitialStakeVaultRT)
	}
	if cfg.InitialDistoVaultRT != 0.5 {
		t.Errorf("InitialDistoVaultRT = %f, want 0.5", cfg.InitialDistoVaultRT)
	}
	if cfg.GenesisBootstrapStakePct != 0.80 {
		t.Errorf("GenesisBootstrapStakePct = %f, want 0.80", cfg.GenesisBootstrapStakePct)
	}
	if cfg.GenesisBootstrapDistoPct != 0.20 {
		t.Errorf("GenesisBootstrapDistoPct = %f, want 0.20", cfg.GenesisBootstrapDistoPct)
	}
	if cfg.LoanPolicy != LoanPolicyTorqBumpImmediate {
		t.Errorf("LoanPolicy = %q, want %q", cfg.LoanPolicy, LoanPolicyTorqBumpImmediate)
	}
	if !cfg.TorqBumpEnabled {
		t.Errorf("TorqBumpEnabled = %v, want true", cfg.TorqBumpEnabled)
	}
	if cfg.TorqBumpThresholdRatio != 0.05 {
		t.Errorf("TorqBumpThresholdRatio = %f, want 0.05", cfg.TorqBumpThresholdRatio)
	}
	if cfg.TorqBumpAmountRT != 0.1 {
		t.Errorf("TorqBumpAmountRT = %f, want 0.1", cfg.TorqBumpAmountRT)
	}
	if cfg.TorqBumpDelayHours != 48 {
		t.Errorf("TorqBumpDelayHours = %d, want 48", cfg.TorqBumpDelayHours)
	}
	if cfg.TorqBumpThresholdPct != 0.25 {
		t.Errorf("TorqBumpThresholdPct = %f, want 0.25", cfg.TorqBumpThresholdPct)
	}
	if cfg.NatsRetries != 5 {
		t.Errorf("NatsRetries = %d, want 5", cfg.NatsRetries)
	}
	if cfg.NatsBackoffBase != 200*time.Millisecond {
		t.Errorf("NatsBackoffBase = %v, want 200ms", cfg.NatsBackoffBase)
	}
	if cfg.LogLevel != "debug" {
		t.Errorf("LogLevel = %q, want %q", cfg.LogLevel, "debug")
	}
	if cfg.DamID != "distodam-test" {
		t.Errorf("DamID = %q, want %q", cfg.DamID, "distodam-test")
	}
}

// TestValidate_BootstrapPercentagesSum validates bootstrap percentages must sum to 1.0.
func TestValidate_BootstrapPercentagesSum(t *testing.T) {
	clearEnv()

	tests := []struct {
		name      string
		stakePct  float64
		distoPct  float64
		wantError bool
	}{
		{"valid_95_5", 0.95, 0.05, false},
		{"valid_80_20", 0.80, 0.20, false},
		{"valid_50_50", 0.50, 0.50, false},
		{"invalid_sum_low", 0.40, 0.40, true},
		{"invalid_sum_high", 0.60, 0.60, true},
		{"invalid_negative", -0.10, 1.10, true},
		{"invalid_over_1", 0.60, 0.50, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cfg := &Config{
				HTTPPort:                 "8082",
				NatsURL:                  "nats://nats:4222",
				MintBatchesTopic:         "mint.batches",
				ContractsApprovedTopic:   "contracts.approved",
				ContractsFundedTopic:     "contracts.funded",
				GenesisBootstrapStakePct: tt.stakePct,
				GenesisBootstrapDistoPct: tt.distoPct,
				LoanPolicy:               LoanPolicyNatural,
				TorqBumpDelayHours:       24,
				TorqBumpThresholdPct:     0.30,
				NatsBackoffBase:          100 * time.Millisecond,
				LogLevel:                 "info",
				DamID:                    "test",
			}

			err := cfg.Validate()
			if tt.wantError && err == nil {
				t.Errorf("Validate() should fail for %s, got nil", tt.name)
			}
			if !tt.wantError && err != nil {
				t.Errorf("Validate() should pass for %s, got %v", tt.name, err)
			}
		})
	}
}

// TestValidate_LoanPolicy validates loan policy must be one of the allowed values.
func TestValidate_LoanPolicy(t *testing.T) {
	clearEnv()

	tests := []struct {
		policy    LoanPolicy
		wantError bool
	}{
		{LoanPolicyNatural, false},
		{LoanPolicyTorqBumpImmediate, false},
		{LoanPolicyTorqBumpThreshold, false},
		{LoanPolicyTorqBumpDelayed, false},
		{LoanPolicy("invalid"), true},
		{LoanPolicy(""), true},
	}

	for _, tt := range tests {
		t.Run(string(tt.policy), func(t *testing.T) {
			cfg := &Config{
				HTTPPort:                 "8082",
				NatsURL:                  "nats://nats:4222",
				MintBatchesTopic:         "mint.batches",
				ContractsApprovedTopic:   "contracts.approved",
				ContractsFundedTopic:     "contracts.funded",
				GenesisBootstrapStakePct: 0.95,
				GenesisBootstrapDistoPct: 0.05,
				LoanPolicy:               tt.policy,
				TorqBumpDelayHours:       24,
				TorqBumpThresholdPct:     0.30,
				NatsBackoffBase:          100 * time.Millisecond,
				LogLevel:                 "info",
				DamID:                    "test",
			}

			err := cfg.Validate()
			if tt.wantError && err == nil {
				t.Errorf("Validate() should fail for policy %q, got nil", tt.policy)
			}
			if !tt.wantError && err != nil {
				t.Errorf("Validate() should pass for policy %q, got %v", tt.policy, err)
			}
		})
	}
}

// TestValidate_TorqBumpThresholds validates Torq bump thresholds are in valid ranges.
func TestValidate_TorqBumpThresholds(t *testing.T) {
	clearEnv()

	tests := []struct {
		name         string
		delayHours   int
		thresholdPct float64
		wantError    bool
	}{
		{"valid_24h_30pct", 24, 0.30, false},
		{"valid_12h_10pct", 12, 0.10, false},
		{"valid_96h_50pct", 96, 0.50, false},
		{"invalid_delay_low", 6, 0.30, true},
		{"invalid_delay_high", 120, 0.30, true},
		{"invalid_threshold_low", 24, 0.05, true},
		{"invalid_threshold_high", 24, 0.60, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cfg := &Config{
				HTTPPort:                 "8082",
				NatsURL:                  "nats://nats:4222",
				MintBatchesTopic:         "mint.batches",
				ContractsApprovedTopic:   "contracts.approved",
				ContractsFundedTopic:     "contracts.funded",
				GenesisBootstrapStakePct: 0.95,
				GenesisBootstrapDistoPct: 0.05,
				LoanPolicy:               LoanPolicyNatural,
				TorqBumpDelayHours:       tt.delayHours,
				TorqBumpThresholdPct:     tt.thresholdPct,
				NatsBackoffBase:          100 * time.Millisecond,
				LogLevel:                 "info",
				DamID:                    "test",
			}

			err := cfg.Validate()
			if tt.wantError && err == nil {
				t.Errorf("Validate() should fail, got nil")
			}
			if !tt.wantError && err != nil {
				t.Errorf("Validate() should pass, got %v", err)
			}
		})
	}
}

// TestValidate_WaterMarks validates water mark ordering.
func TestValidate_WaterMarks(t *testing.T) {
	clearEnv()

	tests := []struct {
		name      string
		stakeLow  float64
		stakeHigh float64
		distoLow  float64
		distoHigh float64
		wantError bool
	}{
		{"valid", 0.001, 0.01, 0.0005, 0.005, false},
		{"stake_high_below_low", 0.01, 0.001, 0.0005, 0.005, true},
		{"disto_high_below_low", 0.001, 0.01, 0.005, 0.0005, true},
		{"negative_stake_low", -0.001, 0.01, 0.0005, 0.005, true},
		{"negative_disto_low", 0.001, 0.01, -0.0005, 0.005, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cfg := &Config{
				HTTPPort:                 "8082",
				NatsURL:                  "nats://nats:4222",
				MintBatchesTopic:         "mint.batches",
				ContractsApprovedTopic:   "contracts.approved",
				ContractsFundedTopic:     "contracts.funded",
				GenesisBootstrapStakePct: 0.95,
				GenesisBootstrapDistoPct: 0.05,
				LoanPolicy:               LoanPolicyNatural,
				StakeLowWaterMarkRT:      tt.stakeLow,
				StakeHighWaterMarkRT:     tt.stakeHigh,
				DistoLowWaterMarkRT:      tt.distoLow,
				DistoHighWaterMarkRT:     tt.distoHigh,
				TorqBumpDelayHours:       24,
				TorqBumpThresholdPct:     0.30,
				NatsBackoffBase:          100 * time.Millisecond,
				LogLevel:                 "info",
				DamID:                    "test",
			}

			err := cfg.Validate()
			if tt.wantError && err == nil {
				t.Errorf("Validate() should fail, got nil")
			}
			if !tt.wantError && err != nil {
				t.Errorf("Validate() should pass, got %v", err)
			}
		})
	}
}

// TestValidate_InvalidLogLevel validates log level must be one of: debug, info, warn, error.
func TestValidate_InvalidLogLevel(t *testing.T) {
	clearEnv()

	cfg := &Config{
		HTTPPort:                 "8082",
		NatsURL:                  "nats://nats:4222",
		MintBatchesTopic:         "mint.batches",
		ContractsApprovedTopic:   "contracts.approved",
		ContractsFundedTopic:     "contracts.funded",
		GenesisBootstrapStakePct: 0.95,
		GenesisBootstrapDistoPct: 0.05,
		LoanPolicy:               LoanPolicyNatural,
		TorqBumpDelayHours:       24,
		TorqBumpThresholdPct:     0.30,
		NatsBackoffBase:          100 * time.Millisecond,
		LogLevel:                 "trace", // Invalid
		DamID:                    "test",
	}

	err := cfg.Validate()
	if err == nil {
		t.Errorf("Validate() should fail for invalid log level, got nil")
	}
}

// TestMustLoad_Panic verifies MustLoad panics on validation failure.
func TestMustLoad_Panic(t *testing.T) {
	clearEnv()
	os.Setenv("GENESIS_BOOTSTRAP_STAKE_PCT", "0.60")
	os.Setenv("GENESIS_BOOTSTRAP_DISTO_PCT", "0.60") // Sum > 1.0
	defer clearEnv()

	defer func() {
		if r := recover(); r == nil {
			t.Errorf("MustLoad() should panic on invalid config, got nil")
		}
	}()

	MustLoad()
}

// TestString verifies String() output contains key config values.
func TestString(t *testing.T) {
	clearEnv()

	cfg, err := Load()
	if err != nil {
		t.Fatalf("Load() failed: %v", err)
	}

	str := cfg.String()

	// Check for key values
	expectedSubstrings := []string{
		"8082",
		"nats://nats:4222",
		"distodam-001",
		"mint.batches",
		"contracts.approved",
		"natural",
		"95.0%",
		"5.0%",
	}

	for _, expected := range expectedSubstrings {
		if !contains(str, expected) {
			t.Errorf("String() missing %q, got:\n%s", expected, str)
		}
	}
}

// Helper: clearEnv clears all DistoDam-related environment variables.
func clearEnv() {
	vars := []string{
		"HTTP_PORT",
		"NATS_URL",
		"MINT_BATCHES_TOPIC",
		"CONTRACTS_APPROVED_TOPIC",
		"CONTRACTS_FUNDED_TOPIC",
		"UBD_REQUESTS_TOPIC",
		"UBD_FUNDED_TOPIC",
		"INITIAL_STAKE_VAULT_RT",
		"INITIAL_DISTO_VAULT_RT",
		"GENESIS_BOOTSTRAP_STAKE_PCT",
		"GENESIS_BOOTSTRAP_DISTO_PCT",
		"DISTODAM_LOAN_POLICY",
		"TORQ_BUMP_ENABLED",
		"TORQ_BUMP_THRESHOLD_RATIO",
		"TORQ_BUMP_AMOUNT_RT",
		"TORQ_BUMP_DELAY_HOURS",
		"TORQ_BUMP_THRESHOLD_PCT",
		"STAKE_LOW_WATER_MARK_RT",
		"STAKE_HIGH_WATER_MARK_RT",
		"DISTO_LOW_WATER_MARK_RT",
		"DISTO_HIGH_WATER_MARK_RT",
		"NATS_RETRIES",
		"NATS_BACKOFF_BASE",
		"LOG_LEVEL",
		"DAM_ID",
		"ADMIN_BEARER_TOKEN",
	}

	for _, v := range vars {
		os.Unsetenv(v)
	}
}

// Helper: contains checks if a string contains a substring.
func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > len(substr) && containsHelper(s, substr))
}

func containsHelper(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
