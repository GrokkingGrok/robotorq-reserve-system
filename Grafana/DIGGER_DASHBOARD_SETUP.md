# Digger Grafana Dashboard Setup

## Overview

This dashboard visualizes **13 Prometheus metrics** from the Digger service, tracking:
- Contract lifecycle (create → stake → execute)
- Printer job orchestration (start → milestones → complete)
- JouleTorqUnit (JTU) generation and storage
- Ore generation and hash batching
- API performance and errors

## Prerequisites

1. **Prometheus** scraping Digger at `http://localhost:3030/metrics`
2. **Grafana** connected to Prometheus datasource
3. **Digger service** running with metrics enabled

---

## Step 1: Configure Prometheus Scraping

Edit `prometheus.yml` to add Digger scrape target:

```yaml
scrape_configs:
  - job_name: 'digger'
    scrape_interval: 5s
    static_configs:
      - targets: ['digger:3030']  # Docker service name
        labels:
          service: 'digger'
```

**Or if running locally** (not in Docker):

```yaml
scrape_configs:
  - job_name: 'digger'
    scrape_interval: 5s
    static_configs:
      - targets: ['localhost:3030']
        labels:
          service: 'digger'
```

Restart Prometheus:

```powershell
docker-compose restart prometheus
```

**Verify scraping**:
1. Open Prometheus UI: http://localhost:9090
2. Go to **Status → Targets**
3. Check that `digger` target is **UP** (green)

---

## Step 2: Import Dashboard to Grafana

### Method A: Via Grafana UI

1. Open Grafana: http://localhost:3000
2. Login (default: admin/admin)
3. Click **Dashboards** → **New** → **Import**
4. Click **Upload JSON file**
5. Select `Grafana/digger-dashboard.json`
6. Select **Prometheus** as datasource
7. Click **Import**

### Method B: Via Grafana API

```powershell
$dashboard = Get-Content "Grafana/digger-dashboard.json" -Raw
Invoke-RestMethod `
  -Uri "http://localhost:3000/api/dashboards/db" `
  -Method Post `
  -Headers @{ "Authorization" = "Bearer YOUR_API_KEY" } `
  -ContentType "application/json" `
  -Body $dashboard
```

**Get API Key**:
1. Grafana → Settings → API Keys
2. Click **New API Key**
3. Name: "Dashboard Import", Role: Admin
4. Copy key and replace `YOUR_API_KEY` above

---

## Step 3: Verify Dashboard

Open the dashboard in Grafana. You should see:

### Contract Lifecycle Section
- **Active Contracts**: Current gauge (0 or 1)
- **Contracts Created**: Total counter
- **Contracts Staked**: Total counter
- **Contracts Executed**: Total counter
- **Creation Rate**: Graph (contracts/sec)
- **Execution Rate**: Graph (executions/sec)

### Printer Activity Section
- **Jobs Started**: Total counter
- **Milestones Reported**: Total counter (should be 4x jobs)
- **Jobs Completed**: Total counter
- **Milestone Rate**: Graph (milestones/sec)
- **Completion Rate**: Graph (jobs/sec)

### JTU Generation Section
- **Total JTUs Generated**: Counter (40,000 per contract)
- **JTUs Stored**: Counter (same as generated)
- **Generation Rate**: Graph (JTUs/sec)
- **Storage Rate**: Graph (ops/sec)

### Ore Generation Section
- **Total Ore**: Counter (400 RT per contract)
- **Hash Batches Sent**: Counter (133 batches per contract)
- **Total Hashes Sent**: Counter (40,000 per contract)
- **Ore Rate**: Graph (RT/sec)
- **Batch Rate**: Graph (batches/sec)

### API Performance Section
- **Request Duration**: Graph (p50, p95, p99 latency)
- **Error Rate**: Graph (errors/sec)

### System Health Section
- **Metric Summary**: Table of all key counters

---

## Step 4: Test with Mock Printer

Run a test to populate the dashboard:

```powershell
python scripts/test_mock_printer.py
```

**Expected Metrics After 1 Test**:
- Active Contracts: 0 (completed)
- Contracts Created: 1
- Contracts Executed: 1
- Jobs Completed: 1
- Milestones Reported: 4
- JTUs Generated: 40,000
- Ore Generated: 400 RT
- Hash Batches: 133

**Expected Graphs**:
- Spikes in rates during execution (0-20 seconds)
- Flat lines after completion

---

## Dashboard Sections Explained

### 1. Contract Lifecycle

Tracks the three-phase contract flow:
1. **Create**: Trust creates contract
2. **Stake**: Digger pays 0.05 RT stake
3. **Execute**: Printer job initiated

**Key Metric**: `digger_contracts_active` - Should be 0 when idle, 1 during execution

### 2. Printer Activity

Monitors physical token printing:
- **Job Start**: Printer acknowledges work
- **Milestones**: Progress reports (4 per job = 25%, 50%, 75%, 100%)
- **Job Complete**: Final confirmation

**Key Metric**: `digger_printer_milestones_reported_total` - Should be 4x `jobs_completed_total`

### 3. JTU Generation

JouleTorqUnits are atomic value tokens:
- **Generated**: Created from contract parameters
- **Stored**: Persisted to SQLite database

**Formula**: 10,000 tokens × 4 milestones = 40,000 JTUs per contract

### 4. Ore Generation

Ore is batched JTUs sent to Refinery:
- **Ore**: 300 JTUs = 1 ore packet = 10 RT
- **Batches**: 300 hashes per batch, sent to NATS

**Formula**: 40,000 JTUs ÷ 100 JTUs/RT = 400 RT ore per contract

### 5. API Performance

HTTP endpoint latency and errors:
- **Duration**: Histogram showing p50/p95/p99 latency
- **Errors**: Counter of failed requests

**Target**: p99 < 100ms, 0 errors

---

## Troubleshooting

### Issue: Dashboard shows "No data"

**Check 1**: Prometheus scraping Digger

```powershell
# Verify Prometheus targets
curl http://localhost:9090/api/v1/targets
# Should show "digger" target with state="up"
```

**Check 2**: Digger metrics endpoint responding

```powershell
curl http://localhost:3030/metrics
# Should return Prometheus metrics
```

**Check 3**: Grafana datasource configured

1. Grafana → Connections → Data sources
2. Click **Prometheus**
3. URL should be `http://prometheus:9090` (Docker) or `http://localhost:9090` (local)
4. Click **Save & test** → Should show "Data source is working"

### Issue: Metrics are all zero

**Solution**: Run a test to generate activity

```powershell
python scripts/test_mock_printer.py
```

### Issue: Rate graphs are flat

**Cause**: Rates calculated over 1-minute windows, need sustained activity

**Solution**: Use instant values for testing:
- Change `rate(digger_contracts_created_total[1m])` to `digger_contracts_created_total`
- Edit panel → Query → Remove `rate()` function

### Issue: p50/p95/p99 graphs error

**Cause**: Histogram metrics need bucket aggregation

**Check**: Verify histogram metrics exist:

```powershell
curl http://localhost:3030/metrics | Select-String "digger_api_request_duration_bucket"
```

Should show multiple bucket lines like:
```
digger_api_request_duration_bucket{le="0.005"} 10
digger_api_request_duration_bucket{le="0.01"} 15
...
```

---

## Dashboard Refresh Rate

Set to **5 seconds** for real-time monitoring during development.

For production:
1. Click dashboard settings (gear icon)
2. Change **Refresh** to 30s or 1m
3. Save dashboard

---

## Adding Alerts

To create alerts for critical metrics:

### Example: Alert on Contract Execution Failures

1. Edit "Contracts Executed" panel
2. Click **Alert** tab
3. Create alert rule:
   - **Condition**: `digger_contracts_executed_total < digger_contracts_created_total - 1`
   - **For**: 5m
   - **Message**: "Contract execution lagging behind creation"
4. Configure notification channel (Slack, email, etc.)

### Example: Alert on High Error Rate

1. Edit "API Error Rate" panel
2. Create alert:
   - **Condition**: `rate(digger_api_errors_total[1m]) > 0.1`
   - **For**: 1m
   - **Message**: "Digger API errors exceeding threshold"

---

## Integration with Existing Dashboards

This dashboard complements:
- **`torq-observability-dashboard.json`**: Trust + DistoDam metrics
- **`robotorq-phase3-dashboard.json`**: Phase 3 pipeline metrics
- **`phase5-pipeline-dashboard.json`**: Phase 5 complete flow

**Suggested Layout**:
1. Create a **Playlist** in Grafana
2. Add all 4 dashboards
3. Set rotation interval (30 seconds)
4. Display on monitoring screen

---

## Metrics Reference

| Metric | Type | Description |
|--------|------|-------------|
| `digger_contracts_created_total` | Counter | Total contracts created |
| `digger_contracts_staked_total` | Counter | Total contracts staked |
| `digger_contracts_executed_total` | Counter | Total contracts executed |
| `digger_contracts_active` | Gauge | Currently active contracts (0 or 1) |
| `digger_printer_jobs_started_total` | Counter | Total printer jobs started |
| `digger_printer_milestones_reported_total` | Counter | Total milestones reported (4 per job) |
| `digger_printer_jobs_completed_total` | Counter | Total jobs completed |
| `digger_jtus_generated_total` | Counter | Total JTUs generated |
| `digger_jtus_stored_total` | Counter | Total JTUs stored in DB |
| `digger_ore_generated_total` | Counter | Total ore generated (RT) |
| `digger_hash_batches_sent_total` | Counter | Total hash batches sent to NATS |
| `digger_hashes_sent_total` | Counter | Total hashes sent |
| `digger_api_request_duration` | Histogram | API request latency distribution |
| `digger_api_errors_total` | Counter | Total API errors |

---

## Next Steps

1. **Set up alerting** for critical thresholds
2. **Create Playlist** with all RoboTorq dashboards
3. **Export dashboard** for version control (already done ✅)
4. **Add annotations** for deployments/incidents
5. **Configure retention** in Prometheus (default: 15 days)

---

## Resources

- Prometheus Docs: https://prometheus.io/docs/
- Grafana Docs: https://grafana.com/docs/grafana/latest/
- PromQL Guide: https://prometheus.io/docs/prometheus/latest/querying/basics/
- Grafana Alerting: https://grafana.com/docs/grafana/latest/alerting/

---

**Dashboard Version**: 1.0.0  
**Created**: [Current Date]  
**Compatible with**: Digger v0.1.0+, Prometheus 2.x, Grafana 9.x+
