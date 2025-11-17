# NATS Integration Manual Test Guide

## Prerequisites
- Docker running
- Rust toolchain installed
- curl or PowerShell Invoke-RestMethod

---

## Step 1: Start NATS

```powershell
cd c:\Users\Jon\Documents\Project-Asimov\robotorq-network
docker-compose up -d nats
```

Verify NATS is running:
```powershell
docker ps | Select-String "nats"
```

Expected output:
```
robotorq-network-nats-1   ... Up ...
```

---

## Step 2: Subscribe to NATS (in separate terminal)

Open a **new terminal** and run:

```powershell
docker exec -it robotorq-network-nats-1 nats sub "ore.batch"
```

Leave this running - it will show messages as they arrive.

---

## Step 3: Start Digger (in separate terminal)

Open **another new terminal** and run:

```powershell
cd c:\Users\Jon\Documents\Project-Asimov\robotorq-network\src\digger

# Set environment variables
$env:DIGGER_ID = "test-robot-001"
$env:NATS_URL = "nats://localhost:4222"
$env:HTTP_PORT = "9000"
$env:BATCH_INTERVAL_SEC = "10"  # Send hashes every 10 seconds (faster for testing)

# Run Digger
cargo run
```

Expected output:
```
🤖 Digger v0.2.0 starting...
Digger ID: test-robot-001
HTTP Port: 9000
Storage Path: ...
Batch Interval: 10 seconds
NATS URL: nats://localhost:4222
🔌 Connecting to NATS...
✅ Connected to NATS at nats://localhost:4222
🚀 Hash sender task started (interval: 10s)
🚀 Digger HTTP server listening on 0.0.0.0:9000
✅ Server ready with full API!
```

---

## Step 4: Create Contract (in original terminal)

```powershell
# Create contract
$body = @{
    contract_id = "nats-test-001"
    torq = 100.0
    robo_stake = 5.0
    milestones = 10
    power_watts = 2000.0
} | ConvertTo-Json

Invoke-RestMethod `
    -Uri "http://localhost:9000/contracts/create" `
    -Method POST `
    -Body $body `
    -ContentType "application/json"
```

Expected response:
```json
{
  "contract_id": "nats-test-001",
  "ore_target": 500.0,
  "message": "Contract created successfully. Pay stake to approve."
}
```

---

## Step 5: Pay Stake (Approve Contract)

```powershell
$stakeBody = @{
    contract_id = "nats-test-001"
} | ConvertTo-Json

Invoke-RestMethod `
    -Uri "http://localhost:9000/contracts/stake" `
    -Method POST `
    -Body $stakeBody `
    -ContentType "application/json"
```

Expected response:
```json
{
  "contract_id": "nats-test-001",
  "approval_status": "StakeApproved",
  "message": "Stake paid, contract approved for execution"
}
```

---

## Step 6: Execute Contract (Generate JTUs)

```powershell
$executeBody = @{
    contract_id = "nats-test-001"
    duration_seconds = 5  # 5 seconds of work
} | ConvertTo-Json

Invoke-RestMethod `
    -Uri "http://localhost:9000/contracts/execute" `
    -Method POST `
    -Body $executeBody `
    -ContentType "application/json"
```

Expected response:
```json
{
  "contract_id": "nats-test-001",
  "jtus_generated": 10000,
  "ore_generated": 100.0,
  "ore_target": 500.0,
  "progress_percent": 20.0,
  "message": "Generated 100 RT of 500 RT target (20% complete)"
}
```

**In Digger logs**, you should see:
```
Generated 10000 JTUs for contract nats-test-001
Inserted 10000 JTUs into storage
```

---

## Step 7: Wait for Hash Transmission

After **10 seconds** (the BATCH_INTERVAL_SEC), check the **NATS subscriber terminal**.

You should see:
```
[#1] Received on "ore.batch"
{
  "contract_id": "nats-test-001",
  "digger_id": "test-robot-001",
  "hashes": [
    "abc123...",
    "def456...",
    ...
  ],
  "hash_count": 10000,
  "timestamp": "2025-11-15T21:30:00Z"
}
```

**In Digger logs**, you should see:
```
📤 Sending hashes for 1 contracts
✅ Sent 10000 hashes for contract nats-test-001 to NATS
```

---

## Step 8: Verify Contract Status

```powershell
Invoke-RestMethod -Uri "http://localhost:9000/contracts/nats-test-001"
```

Expected response includes:
```json
{
  "contract_id": "nats-test-001",
  "approval_status": "StakeApproved",
  "jtu_count": 10000,
  "last_hash_send": 1700000000  // Unix timestamp
}
```

---

## Success Criteria

✅ **PASS** if:
1. NATS subscriber receives hash batch message
2. Message contains correct `contract_id`, `digger_id`, and `hash_count`
3. Digger logs show "Sent X hashes to NATS"
4. Contract state shows `last_hash_send` timestamp updated

❌ **FAIL** if:
- No message appears in NATS subscriber
- Digger logs show NATS publish errors
- `last_hash_send` is null

---

## Cleanup

```powershell
# Stop Digger (Ctrl+C in Digger terminal)
# Stop NATS subscriber (Ctrl+C in subscriber terminal)

# Stop Docker containers
cd c:\Users\Jon\Documents\Project-Asimov\robotorq-network
docker-compose down
```

---

## Troubleshooting

### "Failed to connect to NATS"
- Check NATS is running: `docker ps | Select-String nats`
- Check NATS URL: Should be `nats://localhost:4222` for local testing
- Check firewall isn't blocking port 4222

### "No contracts ready to send hashes"
- Make sure you **paid stake** (contract must be approved)
- Wait for full batch interval (10 seconds in this test)
- Check contract status to verify `approval_status: "StakeApproved"`

### No messages in NATS subscriber
- Verify subscriber is listening to correct subject: `ore.batch`
- Check Digger logs for "Sent X hashes" confirmation
- Ensure batch interval has elapsed

---

## Next Steps

Once NATS integration is verified:
1. **Test with Refinery**: Start Refinery service to consume hash batches
2. **End-to-end flow**: Digger → NATS → Refinery → Mint
3. **Performance testing**: Multiple contracts, high JTU volume
4. **Phase 4**: Add robot identity and Falcon-1024 signatures
5. **Phase 6**: Implement TOON binary encoding (60% bandwidth savings)
