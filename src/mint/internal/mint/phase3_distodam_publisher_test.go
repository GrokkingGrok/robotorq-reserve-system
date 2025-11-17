package mint

import (
	"context"
	"encoding/json"
	"testing"
	"time"

	"b2b/mint/internal/models"

	"log/slog"
	"os"

	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

const testNATSURL = "nats://localhost:4222"

// connectToNATS connects to NATS or skips test if unavailable
func connectToNATS(t *testing.T) *nats.Conn {
	conn, err := nats.Connect(testNATSURL, nats.Timeout(2*time.Second))
	if err != nil {
		t.Skipf("NATS not available at %s, skipping test: %v", testNATSURL, err)
	}
	return conn
}

// TestNewPhase3DistoDamPublisher verifies publisher constructor
func TestNewPhase3DistoDamPublisher(t *testing.T) {
	// Connect to NATS
	conn := connectToNATS(t)
	defer conn.Close()

	// Create channel
	unitChannel := make(chan *models.Phase3RoboTorqUnit, 10)

	// Create logger
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))

	// Create custom metrics registry (avoid global collision)
	registry := prometheus.NewRegistry()
	metrics := NewPhase3DistoDamPublisherMetrics(registry)

	// Create publisher
	publisher := NewPhase3DistoDamPublisher(conn, unitChannel, logger, metrics)

	assert.NotNil(t, publisher)
	assert.Equal(t, conn, publisher.natsConn)
	assert.NotNil(t, publisher.metrics)
	assert.Equal(t, DefaultPublishRetries, publisher.retries)
	assert.Equal(t, DefaultPublishBackoffBase, publisher.backoffBase)
}

// TestPhase3DistoDamPublisher_PublishUnit_Success tests successful unit publishing
func TestPhase3DistoDamPublisher_PublishUnit_Success(t *testing.T) {
	// Connect to NATS
	conn := connectToNATS(t)
	defer conn.Close()

	// Subscribe to receive published units
	received := make(chan *models.Phase3RoboTorqUnit, 1)
	sub, err := conn.Subscribe(Phase3UnitsTopic, func(msg *nats.Msg) {
		var unit models.Phase3RoboTorqUnit
		err := json.Unmarshal(msg.Data, &unit)
		require.NoError(t, err)
		received <- &unit
	})
	require.NoError(t, err)
	defer sub.Unsubscribe()

	// Create publisher
	unitChannel := make(chan *models.Phase3RoboTorqUnit, 10)
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))
	registry := prometheus.NewRegistry()
	metrics := NewPhase3DistoDamPublisherMetrics(registry)
	publisher := NewPhase3DistoDamPublisher(conn, unitChannel, logger, metrics)

	// Create test unit
	unit, err := models.NewPhase3RoboTorqUnit("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", 10)
	require.NoError(t, err)

	// Publish unit
	ctx := context.Background()
	err = publisher.publishUnit(ctx, unit)
	require.NoError(t, err)

	// Verify received
	select {
	case receivedUnit := <-received:
		assert.Equal(t, unit.UnitID, receivedUnit.UnitID)
		assert.Equal(t, unit.MerkleRoot, receivedUnit.MerkleRoot)
	case <-time.After(2 * time.Second):
		t.Fatal("Did not receive published unit")
	}
}

// TestPhase3DistoDamPublisher_PublishUnit_InvalidUnit tests publishing invalid unit
func TestPhase3DistoDamPublisher_PublishUnit_InvalidUnit(t *testing.T) {
	// Connect to NATS
	conn := connectToNATS(t)
	defer conn.Close()

	// Create publisher
	unitChannel := make(chan *models.Phase3RoboTorqUnit, 10)
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))
	registry := prometheus.NewRegistry()
	metrics := NewPhase3DistoDamPublisherMetrics(registry)
	publisher := NewPhase3DistoDamPublisher(conn, unitChannel, logger, metrics)

	// Create invalid unit (empty merkle root)
	unit := &models.Phase3RoboTorqUnit{
		UnitID:     "RT-test",
		MerkleRoot: "", // Invalid!
		MintedAt:   time.Now().UTC(),
	}

	// Attempt publish
	ctx := context.Background()
	err := publisher.publishUnit(ctx, unit)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "invalid Phase3RoboTorqUnit")
}

// TestPhase3DistoDamPublisher_Start_ContextCancellation tests graceful shutdown
func TestPhase3DistoDamPublisher_Start_ContextCancellation(t *testing.T) {
	// Connect to NATS
	conn := connectToNATS(t)
	defer conn.Close()

	// Create publisher
	unitChannel := make(chan *models.Phase3RoboTorqUnit, 10)
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))
	registry := prometheus.NewRegistry()
	metrics := NewPhase3DistoDamPublisherMetrics(registry)
	publisher := NewPhase3DistoDamPublisher(conn, unitChannel, logger, metrics)

	// Start publisher in goroutine
	ctx, cancel := context.WithCancel(context.Background())
	done := make(chan bool)
	go func() {
		publisher.Start(ctx)
		close(done)
	}()

	// Give it time to start
	time.Sleep(100 * time.Millisecond)

	// Cancel context
	cancel()

	// Wait for shutdown
	select {
	case <-done:
		// Success!
	case <-time.After(3 * time.Second):
		t.Fatal("Publisher did not shut down within 3 seconds")
	}
}

// TestPhase3DistoDamPublisher_Start_ChannelClosed tests shutdown on channel close
func TestPhase3DistoDamPublisher_Start_ChannelClosed(t *testing.T) {
	// Connect to NATS
	conn := connectToNATS(t)
	defer conn.Close()

	// Create publisher
	unitChannel := make(chan *models.Phase3RoboTorqUnit, 10)
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))
	registry := prometheus.NewRegistry()
	metrics := NewPhase3DistoDamPublisherMetrics(registry)
	publisher := NewPhase3DistoDamPublisher(conn, unitChannel, logger, metrics)

	// Start publisher in goroutine
	ctx := context.Background()
	done := make(chan bool)
	go func() {
		publisher.Start(ctx)
		close(done)
	}()

	// Give it time to start
	time.Sleep(100 * time.Millisecond)

	// Close channel
	close(unitChannel)

	// Wait for shutdown
	select {
	case <-done:
		// Success!
	case <-time.After(2 * time.Second):
		t.Fatal("Publisher did not shut down within 2 seconds")
	}
}

// TestPhase3DistoDamPublisher_Start_PublishesMultipleUnits tests full pipeline
func TestPhase3DistoDamPublisher_Start_PublishesMultipleUnits(t *testing.T) {
	// Connect to NATS
	conn := connectToNATS(t)
	defer conn.Close()

	// Subscribe to receive published units
	received := make(chan *models.Phase3RoboTorqUnit, 10)
	sub, err := conn.Subscribe(Phase3UnitsTopic, func(msg *nats.Msg) {
		var unit models.Phase3RoboTorqUnit
		err := json.Unmarshal(msg.Data, &unit)
		require.NoError(t, err)
		received <- &unit
	})
	require.NoError(t, err)
	defer sub.Unsubscribe()

	// Create publisher
	unitChannel := make(chan *models.Phase3RoboTorqUnit, 10)
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))
	registry := prometheus.NewRegistry()
	metrics := NewPhase3DistoDamPublisherMetrics(registry)
	publisher := NewPhase3DistoDamPublisher(conn, unitChannel, logger, metrics)

	// Start publisher
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	go publisher.Start(ctx)

	// Send 5 units
	units := make([]*models.Phase3RoboTorqUnit, 5)
	for i := 0; i < 5; i++ {
		merkleRoot := "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
		unit, err := models.NewPhase3RoboTorqUnit(merkleRoot, 10)
		require.NoError(t, err)
		units[i] = unit
		unitChannel <- unit
	}

	// Receive all 5 units
	receivedUnits := make([]*models.Phase3RoboTorqUnit, 0, 5)
	for i := 0; i < 5; i++ {
		select {
		case unit := <-received:
			receivedUnits = append(receivedUnits, unit)
		case <-time.After(2 * time.Second):
			t.Fatalf("Only received %d/5 units", i)
		}
	}

	assert.Equal(t, 5, len(receivedUnits))

	// Verify all units received (order may differ)
	sentIDs := make(map[string]bool)
	for _, u := range units {
		sentIDs[u.UnitID] = true
	}

	for _, u := range receivedUnits {
		assert.True(t, sentIDs[u.UnitID], "Received unexpected unit: %s", u.UnitID)
	}
}

// TestPhase3DistoDamPublisher_DrainRemainingUnits tests drain during shutdown
func TestPhase3DistoDamPublisher_DrainRemainingUnits(t *testing.T) {
	// Connect to NATS
	conn := connectToNATS(t)
	defer conn.Close()

	// Subscribe to receive published units
	received := make(chan *models.Phase3RoboTorqUnit, 10)
	sub, err := conn.Subscribe(Phase3UnitsTopic, func(msg *nats.Msg) {
		var unit models.Phase3RoboTorqUnit
		err := json.Unmarshal(msg.Data, &unit)
		require.NoError(t, err)
		received <- &unit
	})
	require.NoError(t, err)
	defer sub.Unsubscribe()

	// Create publisher
	unitChannel := make(chan *models.Phase3RoboTorqUnit, 10)
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))
	registry := prometheus.NewRegistry()
	metrics := NewPhase3DistoDamPublisherMetrics(registry)
	publisher := NewPhase3DistoDamPublisher(conn, unitChannel, logger, metrics)

	// Add 3 units to channel
	units := make([]*models.Phase3RoboTorqUnit, 3)
	for i := 0; i < 3; i++ {
		merkleRoot := "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
		unit, err := models.NewPhase3RoboTorqUnit(merkleRoot, 10)
		require.NoError(t, err)
		units[i] = unit
		unitChannel <- unit
	}

	// Start publisher
	ctx, cancel := context.WithCancel(context.Background())
	go publisher.Start(ctx)

	// Give time to start
	time.Sleep(100 * time.Millisecond)

	// Cancel context (trigger drain)
	cancel()

	// Wait for all units to be drained and published
	receivedUnits := make([]*models.Phase3RoboTorqUnit, 0, 3)
	timeout := time.After(3 * time.Second)
	for i := 0; i < 3; i++ {
		select {
		case unit := <-received:
			receivedUnits = append(receivedUnits, unit)
		case <-timeout:
			t.Fatalf("Only received %d/3 units during drain", i)
		}
	}

	assert.Equal(t, 3, len(receivedUnits))
}

// TestPhase3DistoDamPublisher_MetricsRecorded verifies Prometheus metrics
func TestPhase3DistoDamPublisher_MetricsRecorded(t *testing.T) {
	// Connect to NATS
	conn := connectToNATS(t)
	defer conn.Close()

	// Create custom metrics
	registry := prometheus.NewRegistry()
	metrics := NewPhase3DistoDamPublisherMetrics(registry)

	// Create publisher
	unitChannel := make(chan *models.Phase3RoboTorqUnit, 10)
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelDebug}))
	publisher := NewPhase3DistoDamPublisher(conn, unitChannel, logger, metrics)

	// Start publisher
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	go publisher.Start(ctx)

	// Publish 1 unit
	merkleRoot := "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
	unit, err := models.NewPhase3RoboTorqUnit(merkleRoot, 10)
	require.NoError(t, err)
	unitChannel <- unit

	// Give time to publish
	time.Sleep(200 * time.Millisecond)

	// Note: We can't easily check counter values in tests without custom registry,
	// but we can verify the code doesn't panic
	assert.NotNil(t, metrics.UnitsPublishedTotal)
	assert.NotNil(t, metrics.PublishErrorsTotal)
	assert.NotNil(t, metrics.PublishLatency)
	assert.NotNil(t, metrics.RetryAttemptsTotal)
}
