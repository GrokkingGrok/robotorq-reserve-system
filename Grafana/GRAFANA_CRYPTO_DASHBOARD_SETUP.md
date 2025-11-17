# Grafana Crypto Pipeline Dashboard Setup Guide

**Purpose**: Visualize Phase 4 crypto signatures flowing through the RoboTorq pipeline  
**Time Required**: 10-15 minutes  
**Prerequisites**: All services running (`docker-compose ps` shows all healthy)

---

## Step 1: Access Grafana

1. **Open browser** to: `http://localhost:3001`
2. **Login credentials**:
   - Username: `admin`
   - Password: `admin`
3. **Skip password change** (click "Skip" when prompted)

✅ **Success**: You should see the Grafana home screen

---

## Step 2: Add Prometheus Data Source

1. **Click** the hamburger menu (≡) in top-left
2. **Navigate to**: Connections → Data sources
3. **Click**: "Add data source" (blue button)
4. **Select**: "Prometheus" (should be first option)
5. **Configure**:
   - **Name**: Leave as "Prometheus" (default)
   - **URL**: `http://prometheus:9090`
   - **Scroll down** to bottom
6. **Click**: "Save & test" (green button at bottom)
7. **Verify**: Green checkmark "Successfully queried the Prometheus API"

✅ **Success**: Prometheus data source is now connected

---

## Step 3: Create New Dashboard

1. **Click** the hamburger menu (≡) again
2. **Navigate to**: Dashboards
3. **Click**: "New" → "New Dashboard" (blue button)
4. **Click**: "+ Add visualization"
5. **Select**: "Prometheus" data source

You'll now see the query editor. We'll create 4 panels.

---

## Step 4: Create Panel 1 - Hash Flow Rate

**Purpose**: Shows hashes per second flowing from Digger (with Falcon signatures)

### Configuration:

1. **Query tab** (already open):
   - **Metric**: Click "Metric" dropdown, search for: `refinery_nats_hashes_received_total`
   - **Or paste directly**: 
     ```promql
     rate(refinery_nats_hashes_received_total[1m])
     ```

2. **Panel options** (right sidebar):
   - **Title**: `Hashes Received from Digger (per second)`
   - **Description**: `Rate of hash reception from Digger ore batches with Falcon-1024 signatures`

3. **Visualization** (top-right):
   - Keep "Time series" (default)
   - **Or change to**: "Gauge" for current rate

4. **Click**: "Apply" (top-right blue button)

✅ **Success**: You should see a graph showing hash ingestion rate

---

## Step 5: Create Panel 2 - Merkle Trees Built

**Purpose**: Shows total merkle trees built from Falcon-signed hash batches

### Configuration:

1. **Click**: "+ Add" → "Visualization"
2. **Select**: "Prometheus" data source

3. **Query tab**:
   - **Paste query**:
     ```promql
     refinery_merkle_trees_built_total
     ```

4. **Panel options**:
   - **Title**: `Merkle Trees Built (with Falcon signatures)`
   - **Description**: `Total merkle trees built from Digger hash batches. Each tree = 3600 hashes.`

5. **Visualization**:
   - **Type**: "Stat" (for big number display)
   - **Graph mode**: "Area" (shows trend)
   - **Color mode**: "Background gradient"

6. **Click**: "Apply"

✅ **Success**: You should see total merkle trees count increasing

---

## Step 6: Create Panel 3 - Phase2 Ingots Assembled

**Purpose**: Shows ingots ready for Mint (each contains merkle branch hash)

### Configuration:

1. **Click**: "+ Add" → "Visualization"
2. **Select**: "Prometheus" data source

3. **Query tab**:
   - **Paste query**:
     ```promql
     refinery_phase2_ingots_assembled_total
     ```

4. **Panel options**:
   - **Title**: `Phase2 Ingots Assembled`
   - **Description**: `Total ingots with merkle branch hashes ready for Mint. Each ingot = 1 merkle tree.`

5. **Visualization**:
   - **Type**: "Stat"
   - **Graph mode**: "Area"
   - **Orientation**: "Horizontal"

6. **Standard options**:
   - **Unit**: "short" (whole numbers)
   - **Decimals**: 0

7. **Click**: "Apply"

✅ **Success**: You should see ingot count (should match merkle trees)

---

## Step 7: Create Panel 4 - Hash Queue Depth

**Purpose**: Real-time queue size monitoring (backpressure indicator)

### Configuration:

1. **Click**: "+ Add" → "Visualization"
2. **Select**: "Prometheus" data source

3. **Query tab**:
   - **Paste query**:
     ```promql
     refinery_hash_queue_size
     ```

4. **Panel options**:
   - **Title**: `Current Hash Queue Size`
   - **Description**: `Real-time count of hashes waiting to be processed into merkle trees`

5. **Visualization**:
   - **Type**: "Time series"
   - **Line interpolation**: "Smooth"
   - **Fill opacity**: 10

6. **Thresholds** (in right sidebar):
   - **Base**: Green
   - **Add threshold**: 3000 → Yellow
   - **Add threshold**: 4500 → Red

7. **Click**: "Apply"

✅ **Success**: You should see queue depth over time with color zones

---

## Step 8: Arrange Dashboard

1. **Drag panels** to arrange in grid:
   ```
   ┌─────────────────────┬─────────────────────┐
   │  Hash Flow Rate     │  Merkle Trees Built │
   ├─────────────────────┼─────────────────────┤
   │  Phase2 Ingots      │  Hash Queue Depth   │
   └─────────────────────┴─────────────────────┘
   ```

2. **Resize panels**: Drag corners to adjust size

---

## Step 9: Save Dashboard

1. **Click**: 💾 Save dashboard icon (top-right)
2. **Name**: `RoboTorq Phase 4 Crypto Pipeline`
3. **Folder**: Leave as "General"
4. **Click**: "Save"

✅ **Success**: Dashboard is now saved and accessible

---

## Step 10: Set Refresh Rate

1. **Top-right corner**: Click time range picker (shows "Last 1 hour")
2. **Refresh**: Set to `5s` or `10s` for live updates
3. **Time range**: Set to "Last 15 minutes" to see recent activity

---

## Verification Steps

### Test the Pipeline

1. **Trigger new contract execution**:
   ```powershell
   Invoke-RestMethod -Uri "http://localhost:3030/contracts/create" -Method Post -ContentType "application/json" -Body '{"contract_id":"grafana-test-001","torq":100.0,"robo_stake":0.05,"milestones":1,"power_watts":500.0}'
   
   Invoke-RestMethod -Uri "http://localhost:3030/contracts/stake" -Method Post -ContentType "application/json" -Body '{"contract_id":"grafana-test-001"}'
   
   Invoke-RestMethod -Uri "http://localhost:3030/contracts/execute" -Method Post -ContentType "application/json" -Body '{"contract_id":"grafana-test-001","duration_seconds":10}'
   ```

2. **Watch Grafana dashboard** (wait 5-10 seconds):
   - ✅ **Hash Flow Rate** should spike
   - ✅ **Merkle Trees Built** should increment
   - ✅ **Phase2 Ingots** should increment
   - ✅ **Hash Queue Depth** should fluctuate

3. **Check Digger logs** to confirm Falcon signing:
   ```powershell
   docker logs robotorq-network-digger-1 --tail 20 | Select-String "Falcon"
   ```
   
   Expected: `✅ Sent 5000 hashes for contract grafana-test-001 to NATS (signed with Falcon-1024)`

---

## Bonus: Add Annotations

### Mark Contract Executions

1. **Edit dashboard** (gear icon top-right)
2. **Settings** → **Annotations**
3. **Add annotation query**:
   - **Name**: "Contract Executions"
   - **Data source**: "Prometheus"
   - **Query**: `changes(refinery_nats_batches_received_total[1m]) > 0`
4. **Save dashboard**

Now you'll see vertical lines when new contracts execute!

---

## Troubleshooting

### No Data Showing

**Problem**: Panels show "No data"

**Solution**:
1. Check services: `docker-compose ps` (all should be healthy)
2. Check Prometheus: `http://localhost:9091/targets` (should show refinery as UP)
3. Verify metrics: `curl http://localhost:8081/metrics | Select-String refinery`
4. Execute test contract (see verification steps above)

### Prometheus Connection Failed

**Problem**: "Error reading Prometheus"

**Solution**:
1. Verify data source URL is `http://prometheus:9090` (not localhost)
2. Check Prometheus is running: `docker-compose ps prometheus`
3. Re-save data source

### Metrics Not Updating

**Problem**: Counters stuck at same value

**Solution**:
1. Trigger new contract execution
2. Check Digger logs: `docker logs robotorq-network-digger-1 --tail 50`
3. Check Refinery logs: `docker logs robotorq-network-refinery-1 --tail 50`
4. Verify NATS connectivity: `docker logs robotorq-network-nats-1 --tail 20`

---

## What You're Visualizing

This dashboard shows the **Phase 4 cryptographic proof chain** in action:

1. **Digger** generates JTU hashes → signs batches with **Falcon-1024**
2. **Refinery** receives signed batches → builds **merkle trees**
3. **Refinery** assembles **Phase2 ingots** with merkle branch hashes
4. **Queue** manages backpressure as data flows

### Crypto Components (Not Yet Visualized)

- **Falcon-1024 signatures**: 1302 bytes per batch (validated in integration test ✅)
- **Public keys**: 1793 bytes (Falcon-1024 spec) ✅
- **SPHINCS+ signatures**: Placeholder in Mint (requires 1000 ingots to see)

---

## Next Steps

1. **Export dashboard JSON**: Settings → JSON Model → Copy
2. **Save to repo**: `Grafana/phase4-crypto-pipeline-dashboard.json`
3. **Commit changes**: Add to Phase 4 PR
4. **Document in README**: Add screenshot + dashboard import instructions

---

## Advanced Queries

### Hash Throughput (5-minute average)
```promql
rate(refinery_nats_hashes_received_total[5m])
```

### Merkle Build Performance
```promql
rate(refinery_merkle_build_duration_seconds_sum[1m]) / rate(refinery_merkle_build_duration_seconds_count[1m])
```

### Ingot Assembly Rate
```promql
rate(refinery_phase2_ingots_assembled_total[1m])
```

### NATS Batch Reception Rate
```promql
rate(refinery_nats_batches_received_total[1m])
```

---

**Dashboard Complete!** 🎉

You now have real-time visibility into the Phase 4 crypto pipeline with Falcon-1024 signatures.
