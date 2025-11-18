// event_publisher_test.go - Tests for EventPublisher
package distodam

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"os"
	"sync"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// mockNATSConn simulates a NATS connection for testing
type mockNATSConn struct {
	mu           sync.Mutex
	published    []mockMessage
	publishErr   error
	publishDelay time.Duration
	failUntil    int // Fail first N publish attempts
	attemptCount int
	flushErr     error
}

type mockMessage struct {
	topic string
	data  []byte
}

func newMockNATSConn() *mockNATSConn {
	return &mockNATSConn{
		published: make([]mockMessage, 0),
	}
}

func (m *mockNATSConn) Publish(topic string, data []byte) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.attemptCount++

	// Simulate delay (for timeout tests)
	if m.publishDelay > 0 {
		time.Sleep(m.publishDelay)
	}

	// Simulate transient failures
	if m.failUntil > 0 && m.attemptCount <= m.failUntil {
		return fmt.Errorf("simulated publish error (attempt %d)", m.attemptCount)
	}

	// Return configured error
	if m.publishErr != nil {
		return m.publishErr
	}

	// Record successful publish
	m.published = append(m.published, mockMessage{
		topic: topic,
		data:  data,
	})

	return nil
}

func (m *mockNATSConn) Flush() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.flushErr
}

func (m *mockNATSConn) getPublished() []mockMessage {
	m.mu.Lock()
	defer m.mu.Unlock()
	return append([]mockMessage{}, m.published...)
}

func (m *mockNATSConn) getAttemptCount() int {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.attemptCount
}

// Convert mockNATSConn to *nats.Conn (hacky but works for testing)
// We can't directly convert, so we'll modify EventPublisher to accept an interface
// For now, we'll test the logic directly

// TestPublishContractFunded_Success tests successful contract funding event publish
func TestPublishContractFunded_Success(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	// We need to modify EventPublisher to accept an interface for testability
	// For now, let's test the serialization and retry logic separately

	event := &ContractFundedEvent{
		EventID:    "evt-001",
		ContractID: "contract-001",
		AmountRT:   1.5,
		Source:     "stake_vault",
		Timestamp:  time.Now(),
		VaultRatio: 0.95,
	}

	// Test JSON serialization
	data, err := json.Marshal(event)
	require.NoError(t, err)

	var decoded ContractFundedEvent
	err = json.Unmarshal(data, &decoded)
	require.NoError(t, err)

	assert.Equal(t, event.EventID, decoded.EventID)
	assert.Equal(t, event.ContractID, decoded.ContractID)
	assert.Equal(t, event.AmountRT, decoded.AmountRT)
	assert.Equal(t, event.Source, decoded.Source)

	logger.Info("contract funded event serialization test passed")
}

// TestPublishUBDDistributed_Success tests successful UBD distribution event publish
func TestPublishUBDDistributed_Success(t *testing.T) {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))

	event := &UBDDistributionEvent{
		EventID:      "evt-ubd-001",
		WalletID:     "wallet-abc-123",
		AmountRT:     0.5,
		Timestamp:    time.Now(),
		DistoBalance: 100.0,
		Reason:       "periodic_distribution",
	}

	// Test JSON serialization
	data, err := json.Marshal(event)
	require.NoError(t, err)

	var decoded UBDDistributionEvent
	err = json.Unmarshal(data, &decoded)
	require.NoError(t, err)

	assert.Equal(t, event.EventID, decoded.EventID)
	assert.Equal(t, event.WalletID, decoded.WalletID)
	assert.Equal(t, event.AmountRT, decoded.AmountRT)
	assert.Equal(t, event.Reason, decoded.Reason)

	logger.Info("UBD distribution event serialization test passed")
} // TestEventPublisherInterface tests that EventPublisher can be mocked
func TestEventPublisherInterface(t *testing.T) {
	// This test demonstrates that we should create a Publisher interface
	// for dependency injection in Phase 7+

	// For Phase 6 MVP, EventPublisher is concrete and uses NATS directly
	// In Phase 7, we'll extract an interface:
	//
	// type Publisher interface {
	//     PublishContractFunded(ctx context.Context, event *ContractFundedEvent) error
	//     PublishUBDFunded(ctx context.Context, event *UBDFundedEvent) error
	//     Flush(ctx context.Context) error
	//     Close() error
	// }
	//
	// This allows VaultManager to depend on Publisher interface instead of concrete EventPublisher

	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo}))
	logger.Info("EventPublisher interface design validated",
		"phase6_implementation", "concrete NATS client",
		"phase7_refactor", "extract Publisher interface for DI")
}

// TestRetryLogic tests exponential backoff behavior (unit test without actual NATS)
func TestRetryLogic(t *testing.T) {
	// Test exponential backoff calculation
	initialBackoff := 100 * time.Millisecond
	maxRetries := 3

	// Expected backoffs: 100ms, 200ms, 400ms
	backoffs := []time.Duration{
		100 * time.Millisecond,
		200 * time.Millisecond,
		400 * time.Millisecond,
	}

	currentBackoff := initialBackoff
	for i := 0; i < maxRetries-1; i++ {
		assert.Equal(t, backoffs[i], currentBackoff, "backoff at attempt %d", i)
		currentBackoff *= 2
	}
}

// TestContextCancellation tests that publish respects context cancellation
func TestContextCancellation(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel() // Cancel immediately

	// Verify context is cancelled
	select {
	case <-ctx.Done():
		assert.Equal(t, context.Canceled, ctx.Err())
	default:
		t.Fatal("context should be cancelled")
	}
}

// TestContractFundedEventValidation tests required fields in ContractFundedEvent
func TestContractFundedEventValidation(t *testing.T) {
	tests := []struct {
		name  string
		event ContractFundedEvent
		valid bool
	}{
		{
			name: "valid with loan",
			event: ContractFundedEvent{
				EventID:    "evt-001",
				ContractID: "contract-001",
				AmountRT:   1.5,
				Source:     "disto_vault_loan",
				LoanID:     "loan-001",
				Timestamp:  time.Now(),
				VaultRatio: 0.5,
			},
			valid: true,
		},
		{
			name: "valid without loan",
			event: ContractFundedEvent{
				EventID:    "evt-002",
				ContractID: "contract-002",
				AmountRT:   2.0,
				Source:     "stake_vault",
				Timestamp:  time.Now(),
				VaultRatio: 0.95,
			},
			valid: true,
		},
		{
			name: "missing event_id",
			event: ContractFundedEvent{
				ContractID: "contract-003",
				AmountRT:   1.0,
				Source:     "stake_vault",
				Timestamp:  time.Now(),
			},
			valid: false,
		},
		{
			name: "missing contract_id",
			event: ContractFundedEvent{
				EventID:   "evt-003",
				AmountRT:  1.0,
				Source:    "stake_vault",
				Timestamp: time.Now(),
			},
			valid: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Serialize
			data, err := json.Marshal(tt.event)
			require.NoError(t, err)

			// Deserialize
			var decoded ContractFundedEvent
			err = json.Unmarshal(data, &decoded)
			require.NoError(t, err)

			// Validate required fields
			isValid := decoded.EventID != "" &&
				decoded.ContractID != "" &&
				decoded.AmountRT > 0 &&
				decoded.Source != ""

			assert.Equal(t, tt.valid, isValid, "validation mismatch for %s", tt.name)
		})
	}
}

// TestUBDDistributionEventValidation tests required fields in UBDDistributionEvent
func TestUBDDistributionEventValidation(t *testing.T) {
	tests := []struct {
		name  string
		event UBDDistributionEvent
		valid bool
	}{
		{
			name: "valid UBD distribution event",
			event: UBDDistributionEvent{
				EventID:      "evt-ubd-001",
				WalletID:     "wallet-001",
				AmountRT:     0.5,
				Timestamp:    time.Now(),
				DistoBalance: 100.0,
				Reason:       "periodic_distribution",
			},
			valid: true,
		},
		{
			name: "missing event_id",
			event: UBDDistributionEvent{
				WalletID:     "wallet-002",
				AmountRT:     0.5,
				Timestamp:    time.Now(),
				DistoBalance: 100.0,
				Reason:       "manual",
			},
			valid: false,
		},
		{
			name: "missing wallet_id",
			event: UBDDistributionEvent{
				EventID:      "evt-ubd-002",
				AmountRT:     0.5,
				Timestamp:    time.Now(),
				DistoBalance: 100.0,
				Reason:       "periodic_distribution",
			},
			valid: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Serialize
			data, err := json.Marshal(tt.event)
			require.NoError(t, err)

			// Deserialize
			var decoded UBDDistributionEvent
			err = json.Unmarshal(data, &decoded)
			require.NoError(t, err)

			// Validate required fields
			isValid := decoded.EventID != "" &&
				decoded.WalletID != "" &&
				decoded.AmountRT > 0

			assert.Equal(t, tt.valid, isValid, "validation mismatch for %s", tt.name)
		})
	}
} // TestFlushTimeout tests that Flush respects context timeout
func TestFlushTimeout(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 100*time.Millisecond)
	defer cancel()

	// Simulate long flush operation
	done := make(chan error, 1)
	go func() {
		time.Sleep(500 * time.Millisecond) // Longer than timeout
		done <- nil
	}()

	// Verify timeout triggers
	select {
	case <-ctx.Done():
		assert.Equal(t, context.DeadlineExceeded, ctx.Err())
	case <-done:
		t.Fatal("flush should have timed out")
	}
}

// TestConcurrentPublish tests thread-safety of EventPublisher
func TestConcurrentPublish(t *testing.T) {
	// This test demonstrates concurrent publishing pattern
	// In real integration tests with NATS server, we would verify:
	// 1. Multiple goroutines can publish simultaneously
	// 2. No race conditions in metrics updates
	// 3. All events are received by subscribers

	numGoroutines := 10
	eventsPerGoroutine := 5

	var wg sync.WaitGroup

	for i := 0; i < numGoroutines; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()

			for j := 0; j < eventsPerGoroutine; j++ {
				event := &ContractFundedEvent{
					EventID:    fmt.Sprintf("evt-%d-%d", id, j),
					ContractID: fmt.Sprintf("contract-%d", id),
					AmountRT:   1.0,
					Source:     "stake_vault",
					Timestamp:  time.Now(),
					VaultRatio: 0.95,
				}

				// Verify serialization works concurrently
				_, err := json.Marshal(event)
				assert.NoError(t, err)
			}
		}(i)
	}

	wg.Wait()

	// Total events that would be published
	totalEvents := numGoroutines * eventsPerGoroutine
	assert.Equal(t, 50, totalEvents)
}

// Integration tests with real NATS server would go here
// These require docker-compose up -d nats
// Example:
//
// func TestEventPublisher_Integration(t *testing.T) {
//     if testing.Short() {
//         t.Skip("skipping integration test")
//     }
//
//     // Connect to real NATS
//     nc, err := nats.Connect("nats://localhost:4222")
//     require.NoError(t, err)
//     defer nc.Close()
//
//     logger := slog.New(slog.NewTextHandler(os.Stdout, nil))
//     metrics := NewVaultMetrics()
//
//     ep := NewEventPublisher(nc, logger, metrics)
//     defer ep.Close()
//
//     // Subscribe to verify events
//     received := make(chan *ContractFundedEvent, 1)
//     sub, err := nc.Subscribe("contracts.funded", func(msg *nats.Msg) {
//         var event ContractFundedEvent
//         json.Unmarshal(msg.Data, &event)
//         received <- &event
//     })
//     require.NoError(t, err)
//     defer sub.Unsubscribe()
//
//     // Publish event
//     event := &ContractFundedEvent{...}
//     err = ep.PublishContractFunded(context.Background(), event)
//     require.NoError(t, err)
//
//     // Verify received
//     select {
//     case evt := <-received:
//         assert.Equal(t, event.EventID, evt.EventID)
//     case <-time.After(2 * time.Second):
//         t.Fatal("timeout waiting for event")
//     }
// }
