# Phase 2 Milestone 1 Test - NATS Subscriber
# Tests that Refinery receives and logs hash batches from NATS

Write-Host "[TEST] Phase 2 Milestone 1: NATS Subscriber" -ForegroundColor Cyan
Write-Host ""

# Step 1: Start NATS (should already be running)
Write-Host "[STEP 1] Ensure NATS is running..." -ForegroundColor Yellow
docker-compose up -d nats
Start-Sleep -Seconds 2

# Step 2: Start Refinery in background
Write-Host "[STEP 2] Starting Refinery..." -ForegroundColor Yellow
cd src/refinery
Start-Process -NoNewWindow -FilePath "go" -ArgumentList "run","cmd/refinery/main.go" -RedirectStandardOutput "refinery-output.txt" -RedirectStandardError "refinery-error.txt"
cd ../..
Start-Sleep -Seconds 5

# Step 3: Start Digger in background
Write-Host "[STEP 3] Starting Digger..." -ForegroundColor Yellow
cd src/digger
$env:NATS_URL = "nats://localhost:4222"
$env:BATCH_INTERVAL_SEC = "10"
Start-Process -NoNewWindow -FilePath "cargo" -ArgumentList "run","--release" -RedirectStandardOutput "digger-output.txt" -RedirectStandardError "digger-error.txt"
cd ../..
Start-Sleep -Seconds 10

# Step 4: Execute a small contract
Write-Host "[STEP 4] Executing test contract..." -ForegroundColor Yellow
$body = @{
    contract_id = "milestone1-test"
    duration_seconds = 3
} | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri "http://localhost:9000/execute" -Method Post -Body $body -ContentType "application/json"
    Write-Host "[SUCCESS] Contract executed:" -ForegroundColor Green
    Write-Host ($response | ConvertTo-Json -Depth 3)
} catch {
    Write-Host "[ERROR] Failed to execute contract: $_" -ForegroundColor Red
    exit 1
}

# Step 5: Wait for hash transmission
Write-Host ""
Write-Host "[STEP 5] Waiting 15 seconds for hash transmission..." -ForegroundColor Yellow
Start-Sleep -Seconds 15

# Step 6: Check Refinery logs for hash batch
Write-Host ""
Write-Host "[STEP 6] Checking Refinery logs..." -ForegroundColor Yellow
$refineryLogs = Get-Content "src/refinery/refinery-output.txt" -ErrorAction SilentlyContinue

if ($refineryLogs -match "received hash batch") {
    Write-Host "[SUCCESS] Refinery received hash batch!" -ForegroundColor Green
    
    # Show the relevant log lines
    $refineryLogs | Select-String "received hash batch" | ForEach-Object {
        Write-Host "  $_" -ForegroundColor Cyan
    }
    
    # Check for hash count
    $hashCountLine = $refineryLogs | Select-String "hash_count" | Select-Object -Last 1
    if ($hashCountLine) {
        Write-Host ""
        Write-Host "Hash batch details:" -ForegroundColor Yellow
        Write-Host "  $hashCountLine" -ForegroundColor Cyan
    }
    
    Write-Host ""
    Write-Host "[MILESTONE 1] ✅ PASSED - NATS Subscriber Working!" -ForegroundColor Green
} else {
    Write-Host "[FAILURE] No hash batch received in Refinery logs" -ForegroundColor Red
    Write-Host ""
    Write-Host "Refinery logs (last 20 lines):" -ForegroundColor Yellow
    $refineryLogs | Select-Object -Last 20 | ForEach-Object {
        Write-Host "  $_"
    }
    
    Write-Host ""
    Write-Host "[MILESTONE 1] ❌ FAILED" -ForegroundColor Red
    exit 1
}

# Cleanup
Write-Host ""
Write-Host "[CLEANUP] Stopping services..." -ForegroundColor Yellow
Stop-Process -Name "refinery" -Force -ErrorAction SilentlyContinue
Stop-Process -Name "digger" -Force -ErrorAction SilentlyContinue
docker-compose down

Write-Host ""
Write-Host "[COMPLETE] Test finished successfully!" -ForegroundColor Green
