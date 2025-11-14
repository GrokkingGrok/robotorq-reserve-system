// Package executor communicates with Digger HTTP API to execute funded contracts.
//
// For each funded contract, it:
//  1. Queries GET /robot/status to verify Digger availability
//  2. Sends POST /stake with RoboStake payment
//  3. Updates contract status to "executing"
//
// The executor runs a worker pool to handle multiple contracts concurrently.
package executor

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	"b2b/trust/internal/contract"
	"b2b/trust/internal/metrics"

	"go.uber.org/zap"
)

// RobotStatusResponse matches the JSON from Digger's GET /robot/status
type RobotStatusResponse struct {
	Available       bool    `json:"available"`
	DiggerID        string  `json:"digger_id"`
	CurrentContract *string `json:"current_contract,omitempty"`
}

// StakeRequest matches the JSON for Digger's POST /stake
type StakeRequest struct {
	ContractID string  `json:"contract_id"`
	AmountRT   float64 `json:"amount_rt"`
	Builder    string  `json:"builder"`
}

// Start receives funded contracts from FundSync and processes them
func Start(ctx context.Context, in <-chan *contract.Contract, out chan<- *contract.Contract, logger *zap.Logger, m *metrics.Metrics, workers int) {
	httpClient := &http.Client{
		Timeout: 10 * time.Second,
	}

	for i := 0; i < workers; i++ {
		go func(id int) {
			for {
				select {
				case c := <-in:
					logger.Info("Executing contract",
						zap.String("contract_id", c.ID),
						zap.String("opportunity_id", c.OpportunityID),
						zap.String("digger_url", c.GetDiggerURL()),
						zap.Int("worker", id))

					// Execute the contract by communicating with Digger
					if err := executeContract(httpClient, c, logger); err != nil {
						logger.Error("Failed to execute contract",
							zap.String("contract_id", c.ID),
							zap.Error(err))
						// TODO(enhancement): Add retry logic with exponential backoff
						// and/or dead-letter queue for persistent failures. Current
						// behavior logs and drops failed executions.
						continue
					}

					m.IncExecutions()
					m.IncContractsExecuted()

					// Pass to next stage
					select {
					case out <- c:
					default:
						logger.Warn("Exec channel full, dropping contract",
							zap.String("contract_id", c.ID))
					}
				case <-ctx.Done():
					return
				}
			}
		}(i)
	}
}

// executeContract queries Digger HTTP endpoints and updates contract status
func executeContract(client *http.Client, c *contract.Contract, logger *zap.Logger) error {
	diggerURL := c.GetDiggerURL()
	if diggerURL == "" {
		return fmt.Errorf("contract %s has no digger_url", c.ID)
	}

	// Step 1: GET /robot/status to check availability
	statusURL := fmt.Sprintf("%s/robot/status", diggerURL)
	resp, err := client.Get(statusURL)
	if err != nil {
		return fmt.Errorf("failed to query robot status: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("robot status returned %d: %s", resp.StatusCode, string(body))
	}

	var status RobotStatusResponse
	if err := json.NewDecoder(resp.Body).Decode(&status); err != nil {
		return fmt.Errorf("failed to parse robot status: %w", err)
	}

	if !status.Available {
		return fmt.Errorf("robot %s is not available (current_contract: %v)", status.DiggerID, status.CurrentContract)
	}

	logger.Info("Robot is available",
		zap.String("digger_id", status.DiggerID),
		zap.String("contract_id", c.ID))

	// Step 2: POST /stake to send RoboStake payment
	stakeReq := StakeRequest{
		ContractID: c.ID,
		AmountRT:   c.GetRoboStake(),
		Builder:    c.Builder,
	}

	stakeJSON, err := json.Marshal(stakeReq)
	if err != nil {
		return fmt.Errorf("failed to marshal stake request: %w", err)
	}

	stakeURL := fmt.Sprintf("%s/stake", diggerURL)
	resp, err = client.Post(stakeURL, "application/json", bytes.NewBuffer(stakeJSON))
	if err != nil {
		return fmt.Errorf("failed to POST stake: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusAccepted {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("stake endpoint returned %d: %s", resp.StatusCode, string(body))
	}

	logger.Info("Stake sent to robot",
		zap.String("contract_id", c.ID),
		zap.Float64("amount_rt", c.GetRoboStake()),
		zap.String("digger_id", status.DiggerID))

	// Step 3: Update contract status to "executing"
	c.UpdateStatus("executing")

	return nil
}
