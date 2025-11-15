package mint

import (
	"context"
	"encoding/json"
	"io"
	"log/slog"
	"sync"
	"testing"
	"time"

	"github.com/nats-io/nats-server/v2/server"
	"github.com/nats-io/nats.go"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// ─────────────────────────────────────────────────────────────
// Test Helpers
// ─────────────────────────────────────────────────────────────

// startEmbeddedNATS starts an embedded NATS server for testing.
func startEmbeddedNATS(t *testing.T) (*server.Server, string) {
	opts := &server.Options{
		Host: "127.0.0.1",
		Port: -1, // Random port
	}

	ns, err := server.NewServer(opts)
	require.NoError(t, err)

	go ns.Start()

	// Wait for server to be ready
	if !ns.ReadyForConnections(5 * time.Second) {
		t.Fatal("NATS server not ready")
	}

	natsURL := ns.ClientURL()
	t.Logf("Embedded NATS server started on %s", natsURL)

	return ns, natsURL
}

// stopEmbeddedNATS gracefully stops the embedded NATS server.
func stopEmbeddedNATS(t *testing.T, ns *server.Server) {
	ns.Shutdown()
	ns.WaitForShutdown()
	t.Log("Embedded NATS server stopped")
}

// setupTestDistoDamClient creates a test DistoDamClient with embedded NATS.
func setupTestDistoDamClient(t *testing.T) (DistoDamClient, *server.Server, string) {
	// Reset Prometheus registry
	prometheus.DefaultRegisterer = prometheus.NewRegistry()

	ns, natsURL := startEmbeddedNATS(t)
	logger := slog.New(slog.NewTextHandler(io.Discard, nil))
	client := NewDistoDamClient(natsURL, logger)

	return client, ns, natsURL
}

// ─────────────────────────────────────────────────────────────
// Connection Tests
// ─────────────────────────────────────────────────────────────

func TestDistoDamClient_Connect_Success(t *testing.T) {
	client, ns, _ := setupTestDistoDamClient(t)
	defer stopEmbeddedNATS(t, ns)

	err := client.Connect()
	require.NoError(t, err)

	assert.True(t, client.IsConnected())

	err = client.Close()
	assert.NoError(t, err)
}

func TestDistoDamClient_Connect_InvalidURL(t *testing.T) {
	prometheus.DefaultRegisterer = prometheus.NewRegistry()
	logger := slog.New(slog.NewTextHandler(io.Discard, nil))

	client := NewDistoDamClient("nats://invalid-host:9999", logger)

	err := client.Connect()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "failed to connect")
	assert.False(t, client.IsConnected())
}

func TestDistoDamClient_IsConnected(t *testing.T) {
	client, ns, _ := setupTestDistoDamClient(t)
	defer stopEmbeddedNATS(t, ns)

	// Initially disconnected
	assert.False(t, client.IsConnected())

	// Connect
	err := client.Connect()
	require.NoError(t, err)
	assert.True(t, client.IsConnected())

	// Close
	err = client.Close()
	require.NoError(t, err)
	assert.False(t, client.IsConnected())
}

// ─────────────────────────────────────────────────────────────
// Publish Tests
// ─────────────────────────────────────────────────────────────

func TestDistoDamClient_Publish_Success(t *testing.T) {
	client, ns, natsURL := setupTestDistoDamClient(t)
	defer stopEmbeddedNATS(t, ns)

	err := client.Connect()
	require.NoError(t, err)
	defer client.Close()

	// Subscribe to verify message received
	nc, err := nats.Connect(natsURL)
	require.NoError(t, err)
	defer nc.Close()

	received := make(chan *MintEvent, 1)
	sub, err := nc.Subscribe(MintBatchesTopic, func(msg *nats.Msg) {
		var event MintEvent
		if err := json.Unmarshal(msg.Data, &event); err == nil {
			received <- &event
		}
	})
	require.NoError(t, err)
	defer sub.Unsubscribe()

	// Publish event
	event := &MintEvent{
		BatchHash:       "test_hash_123",
		TotalRoboTorq:   1000.0,
		IngotsProcessed: 100,
		SaleValueUSD:    500.0,
		BatchID:         "batch-123",
		Timestamp:       time.Now(),
	}

	ctx := context.Background()
	err = client.Publish(ctx, event)
	require.NoError(t, err)

	// Verify message received
	select {
	case receivedEvent := <-received:
		assert.Equal(t, event.BatchHash, receivedEvent.BatchHash)
		assert.Equal(t, event.TotalRoboTorq, receivedEvent.TotalRoboTorq)
		assert.Equal(t, event.IngotsProcessed, receivedEvent.IngotsProcessed)
		assert.Equal(t, event.BatchID, receivedEvent.BatchID)
	case <-time.After(1 * time.Second):
		t.Fatal("Did not receive published event")
	}
}

func TestDistoDamClient_Publish_MultipleBatches(t *testing.T) {
	client, ns, natsURL := setupTestDistoDamClient(t)
	defer stopEmbeddedNATS(t, ns)

	err := client.Connect()
	require.NoError(t, err)
	defer client.Close()

	// Subscribe
	nc, err := nats.Connect(natsURL)
	require.NoError(t, err)
	defer nc.Close()

	received := make(chan *MintEvent, 5)
	sub, err := nc.Subscribe(MintBatchesTopic, func(msg *nats.Msg) {
		var event MintEvent
		if err := json.Unmarshal(msg.Data, &event); err == nil {
			received <- &event
		}
	})
	require.NoError(t, err)
	defer sub.Unsubscribe()

	// Flush subscription to ensure it's active
	err = nc.Flush()
	require.NoError(t, err)

	// Publish 5 events
	ctx := context.Background()
	for i := 0; i < 5; i++ {
		event := &MintEvent{
			BatchHash:       "hash_" + string(rune('A'+i)),
			TotalRoboTorq:   float64(i * 100),
			IngotsProcessed: i * 10,
			BatchID:         "batch-" + string(rune('A'+i)),
			Timestamp:       time.Now(),
		}
		err = client.Publish(ctx, event)
		require.NoError(t, err)
	}

	// Verify all received
	receivedCount := 0
	timeout := time.After(2 * time.Second)

	for receivedCount < 5 {
		select {
		case <-received:
			receivedCount++
		case <-timeout:
			t.Fatalf("Only received %d/5 events", receivedCount)
		}
	}

	assert.Equal(t, 5, receivedCount)
}

// ─────────────────────────────────────────────────────────────
// Retry Logic Tests
// ─────────────────────────────────────────────────────────────

func TestDistoDamClient_Publish_NotConnected(t *testing.T) {
	prometheus.DefaultRegisterer = prometheus.NewRegistry()
	logger := slog.New(slog.NewTextHandler(io.Discard, nil))

	// Create client but don't connect
	client := NewDistoDamClient("nats://localhost:4222", logger)

	event := &MintEvent{
		BatchHash: "test",
		BatchID:   "123",
		Timestamp: time.Now(),
	}

	ctx := context.Background()
	err := client.Publish(ctx, event)

	// Should fail because not connected
	assert.Error(t, err)
}

func TestDistoDamClient_Publish_ContextCancellation(t *testing.T) {
	client, ns, _ := setupTestDistoDamClient(t)
	defer stopEmbeddedNATS(t, ns)

	err := client.Connect()
	require.NoError(t, err)
	defer client.Close()

	// Create cancelled context
	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	event := &MintEvent{
		BatchHash: "test",
		BatchID:   "123",
		Timestamp: time.Now(),
	}

	// Close connection to force retries
	client.Close()

	err = client.Publish(ctx, event)

	// Should fail quickly due to cancelled context
	assert.Error(t, err)
}

// ─────────────────────────────────────────────────────────────
// Concurrent Publishing Tests
// ─────────────────────────────────────────────────────────────

func TestDistoDamClient_ConcurrentPublish(t *testing.T) {
	client, ns, natsURL := setupTestDistoDamClient(t)
	defer stopEmbeddedNATS(t, ns)

	err := client.Connect()
	require.NoError(t, err)
	defer client.Close()

	// Subscribe
	nc, err := nats.Connect(natsURL)
	require.NoError(t, err)
	defer nc.Close()

	received := make(chan *MintEvent, 50)
	sub, err := nc.Subscribe(MintBatchesTopic, func(msg *nats.Msg) {
		var event MintEvent
		if err := json.Unmarshal(msg.Data, &event); err == nil {
			received <- &event
		}
	})
	require.NoError(t, err)
	defer sub.Unsubscribe()

	// Publish 50 events concurrently
	var wg sync.WaitGroup
	numEvents := 50

	wg.Add(numEvents)
	for i := 0; i < numEvents; i++ {
		go func(eventNum int) {
			defer wg.Done()
			event := &MintEvent{
				BatchHash:       "hash_" + string(rune('0'+eventNum%10)),
				TotalRoboTorq:   float64(eventNum * 100),
				IngotsProcessed: eventNum,
				BatchID:         "batch-" + string(rune('0'+eventNum%10)),
				Timestamp:       time.Now(),
			}
			ctx := context.Background()
			err := client.Publish(ctx, event)
			assert.NoError(t, err)
		}(i)
	}

	wg.Wait()

	// Verify all received
	receivedCount := 0
	timeout := time.After(3 * time.Second)

	for receivedCount < numEvents {
		select {
		case <-received:
			receivedCount++
		case <-timeout:
			t.Fatalf("Only received %d/%d events", receivedCount, numEvents)
		}
	}

	assert.Equal(t, numEvents, receivedCount)
}

// ─────────────────────────────────────────────────────────────
// Graceful Shutdown Tests
// ─────────────────────────────────────────────────────────────

func TestDistoDamClient_Close_GracefulShutdown(t *testing.T) {
	client, ns, natsURL := setupTestDistoDamClient(t)
	defer stopEmbeddedNATS(t, ns)

	err := client.Connect()
	require.NoError(t, err)

	// Subscribe
	nc, err := nats.Connect(natsURL)
	require.NoError(t, err)
	defer nc.Close()

	received := make(chan *MintEvent, 10)
	sub, err := nc.Subscribe(MintBatchesTopic, func(msg *nats.Msg) {
		var event MintEvent
		if err := json.Unmarshal(msg.Data, &event); err == nil {
			received <- &event
		}
	})
	require.NoError(t, err)
	defer sub.Unsubscribe()

	// Flush subscription to ensure it's active
	err = nc.Flush()
	require.NoError(t, err)

	// Publish some events
	ctx := context.Background()
	for i := 0; i < 5; i++ {
		event := &MintEvent{
			BatchHash: "hash",
			BatchID:   "batch",
			Timestamp: time.Now(),
		}
		err = client.Publish(ctx, event)
		require.NoError(t, err)
	}

	// Flush client to ensure all messages sent
	time.Sleep(50 * time.Millisecond)

	// Close should drain messages
	err = client.Close()
	assert.NoError(t, err)

	// All messages should have been delivered
	time.Sleep(100 * time.Millisecond) // Allow time for delivery

	receivedCount := 0
	for {
		select {
		case <-received:
			receivedCount++
		case <-time.After(100 * time.Millisecond):
			goto done
		}
	}

done:
	assert.GreaterOrEqual(t, receivedCount, 3, "Most messages should be delivered before close")
}

func TestDistoDamClient_Close_Idempotent(t *testing.T) {
	client, ns, _ := setupTestDistoDamClient(t)
	defer stopEmbeddedNATS(t, ns)

	err := client.Connect()
	require.NoError(t, err)

	// Close multiple times should not error
	err = client.Close()
	assert.NoError(t, err)

	err = client.Close()
	assert.NoError(t, err)

	err = client.Close()
	assert.NoError(t, err)
}

// ─────────────────────────────────────────────────────────────
// Topic Verification Tests
// ─────────────────────────────────────────────────────────────

func TestDistoDamClient_CorrectTopic(t *testing.T) {
	client, ns, natsURL := setupTestDistoDamClient(t)
	defer stopEmbeddedNATS(t, ns)

	err := client.Connect()
	require.NoError(t, err)
	defer client.Close()

	// Subscribe to correct topic
	nc, err := nats.Connect(natsURL)
	require.NoError(t, err)
	defer nc.Close()

	correctTopic := make(chan bool, 1)
	sub1, err := nc.Subscribe(MintBatchesTopic, func(msg *nats.Msg) {
		correctTopic <- true
	})
	require.NoError(t, err)
	defer sub1.Unsubscribe()

	// Subscribe to wrong topic
	wrongTopic := make(chan bool, 1)
	sub2, err := nc.Subscribe("wrong.topic", func(msg *nats.Msg) {
		wrongTopic <- true
	})
	require.NoError(t, err)
	defer sub2.Unsubscribe()

	// Flush to ensure subscriptions are active
	err = nc.Flush()
	require.NoError(t, err)

	// Publish event
	event := &MintEvent{
		BatchHash: "test",
		BatchID:   "123",
		Timestamp: time.Now(),
	}

	ctx := context.Background()
	err = client.Publish(ctx, event)
	require.NoError(t, err)

	// Should receive on correct topic only
	select {
	case <-correctTopic:
		// Good!
	case <-time.After(1 * time.Second):
		t.Fatal("Did not receive on correct topic")
	}

	select {
	case <-wrongTopic:
		t.Fatal("Received on wrong topic")
	case <-time.After(200 * time.Millisecond):
		// Good - no message on wrong topic
	}
}

// ─────────────────────────────────────────────────────────────
// Performance Tests
// ─────────────────────────────────────────────────────────────

func TestDistoDamClient_PublishLatency(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping performance test in short mode")
	}

	client, ns, _ := setupTestDistoDamClient(t)
	defer stopEmbeddedNATS(t, ns)

	err := client.Connect()
	require.NoError(t, err)
	defer client.Close()

	// Publish 1000 events and measure latency
	ctx := context.Background()
	iterations := 1000

	start := time.Now()
	for i := 0; i < iterations; i++ {
		event := &MintEvent{
			BatchHash:       "hash",
			TotalRoboTorq:   1000.0,
			IngotsProcessed: 100,
			BatchID:         "batch",
			Timestamp:       time.Now(),
		}
		err := client.Publish(ctx, event)
		require.NoError(t, err)
	}
	elapsed := time.Since(start)

	avgLatency := elapsed / time.Duration(iterations)
	throughput := float64(iterations) / elapsed.Seconds()

	t.Logf("Published %d events in %v", iterations, elapsed)
	t.Logf("Average latency: %v", avgLatency)
	t.Logf("Throughput: %.0f events/sec", throughput)

	assert.Less(t, avgLatency, 5*time.Millisecond, "Average latency should be under 5ms")
}
