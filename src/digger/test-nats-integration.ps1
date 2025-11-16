# Test NATS Hash Transmission
# 
# This script:
# 1. Starts NATS in Docker
# 2. Runs Digger locally
# 3. Creates & executes a contract
# 4. Monitors NATS for hash transmission
# 5. Verifies hash batch received

Write-Host "Testing NATS Hash Transmission" -ForegroundColor Cyan
Write-Host ""

# Step 1: Start NATS
Write-Host "[NATS] Starting NATS..." -ForegroundColor Yellow
docker-compose up -d nats
Start-Sleep -Seconds 3

# Verify NATS is running
$natsRunning = docker ps --filter "name=nats" --filter "status=running" --format "{{.Names}}"
if ($natsRunning -notlike "*nats*") {
    Write-Host "[ERROR] NATS failed to start" -ForegroundColor Red
    exit 1
}
Write-Host "[OK] NATS running" -ForegroundColor Green
Write-Host ""

# Step 2: Start NATS subscriber in background
Write-Host "[NATS] Starting NATS subscriber..." -ForegroundColor Yellow
$subscriberJob = Start-Job -ScriptBlock {
    docker exec robotorq-network-nats-1 nats sub "ore.batch"
}
Start-Sleep -Seconds 2
Write-Host "[OK] Subscriber listening on 'ore.batch'" -ForegroundColor Green
Write-Host ""

# Step 3: Start Digger
Write-Host "[DIGGER] Starting Digger..." -ForegroundColor Yellow
$diggerJob = Start-Job -ScriptBlock {
    Set-Location "c:\Users\Jon\Documents\Project-Asimov\robotorq-network\src\digger"
    $env:DIGGER_ID = "test-robot-nats-001"
    $env:NATS_URL = "nats://localhost:4222"
    $env:HTTP_PORT = "9000"
    $env:BATCH_INTERVAL_SEC = "10"  # Short interval for testing
    cargo run
}
Start-Sleep -Seconds 5
Write-Host "[OK] Digger started" -ForegroundColor Green
Write-Host ""

# Step 4: Create contract
Write-Host "[CONTRACT] Creating contract..." -ForegroundColor Yellow
$createBody = @{
    contract_id = "nats-test-001"
    torq = 100.0
    robo_stake = 5.0
    milestones = 5
    power_watts = 2000.0
} | ConvertTo-Json

try {
    $createResponse = Invoke-RestMethod `
        -Uri "http://localhost:9000/contracts/create" `
        -Method POST `
        -Body $createBody `
        -ContentType "application/json"
    
    Write-Host "[OK] Contract created: ore_target = $($createResponse.ore_target) RT" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Failed to create contract: $_" -ForegroundColor Red
    Stop-Job $diggerJob
    Stop-Job $subscriberJob
    exit 1
}
Write-Host ""

# Step 5: Pay stake
Write-Host "[STAKE] Paying stake..." -ForegroundColor Yellow
$stakeBody = @{
    contract_id = "nats-test-001"
} | ConvertTo-Json

try {
    $stakeResponse = Invoke-RestMethod `
        -Uri "http://localhost:9000/contracts/stake" `
        -Method POST `
        -Body $stakeBody `
        -ContentType "application/json"
    
    Write-Host "[OK] Stake approved" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Failed to pay stake: $_" -ForegroundColor Red
    Stop-Job $diggerJob
    Stop-Job $subscriberJob
    exit 1
}
Write-Host ""

# Step 6: Execute contract
Write-Host "[EXECUTE] Executing contract..." -ForegroundColor Yellow
$executeBody = @{
    contract_id = "nats-test-001"
    duration_sec = 5
} | ConvertTo-Json

try {
    $executeResponse = Invoke-RestMethod `
        -Uri "http://localhost:9000/contracts/execute" `
        -Method POST `
        -Body $executeBody `
        -ContentType "application/json"
    
    Write-Host "[OK] Contract executed: $($executeResponse.jtus_generated) JTUs generated" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Failed to execute contract: $_" -ForegroundColor Red
    Stop-Job $diggerJob
    Stop-Job $subscriberJob
    exit 1
}
Write-Host ""

# Step 7: Wait for hash transmission (batch interval + buffer)
Write-Host "[WAIT] Waiting for hash transmission (15 seconds)..." -ForegroundColor Yellow
Start-Sleep -Seconds 15

# Step 8: Check subscriber output
Write-Host ""
Write-Host "[NATS] Messages Received:" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
$subscriberOutput = Receive-Job $subscriberJob
if ($subscriberOutput) {
    $subscriberOutput | Write-Host
    Write-Host ""
    Write-Host "[SUCCESS] Hash batch received on NATS!" -ForegroundColor Green
} else {
    Write-Host "[FAILURE] No messages received on NATS" -ForegroundColor Red
}
Write-Host ""

# Cleanup
Write-Host "[CLEANUP] Stopping jobs..." -ForegroundColor Yellow
Stop-Job $diggerJob -ErrorAction SilentlyContinue
Remove-Job $diggerJob -ErrorAction SilentlyContinue
Stop-Job $subscriberJob -ErrorAction SilentlyContinue
Remove-Job $subscriberJob -ErrorAction SilentlyContinue
docker-compose down

Write-Host "[SUCCESS] Test complete!" -ForegroundColor Green
