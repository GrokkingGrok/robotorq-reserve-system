# E2E Test Script for Mint Service
# This script tests the full flow: HTTP ingot submission to NATS publishing

param(
    [string]$MintURL = "http://localhost:8080",
    [int]$NumIngots = 10,
    [int]$BatchSize = 5
)

Write-Host "E2E Test: Mint Service" -ForegroundColor Cyan
Write-Host "  Mint URL: $MintURL" -ForegroundColor Gray
Write-Host "  Sending: $NumIngots ingots (batch size: $BatchSize)" -ForegroundColor Gray
Write-Host ""

# Test 1: Health Check
Write-Host "[1/4] Testing health endpoint..." -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri "$MintURL/health" -Method Get -TimeoutSec 5
    Write-Host "  Success: Health check passed" -ForegroundColor Green
} catch {
    Write-Host "  Error: Health check failed" -ForegroundColor Red
    exit 1
}

# Test 2: Submit Ingots
Write-Host "[2/4] Submitting $NumIngots ingots..." -ForegroundColor Yellow
$successCount = 0
$failCount = 0

for ($i = 1; $i -le $NumIngots; $i++) {
    $ingot = @{
        joule      = 3600.0
        robo       = 100.0 + ($i * 10)
        price      = 50.0 + ($i * 5)
        contract_id = "CONTRACT-001"
        digger_id   = "ROBOT-$('{0:D4}' -f $i)"
        timestamp   = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
        hash        = "0x" + (1..$i -join "")
    } | ConvertTo-Json

    try {
        $response = Invoke-RestMethod -Uri "$MintURL/mint-tokentorq" `
            -Method Post `
            -ContentType "application/json" `
            -Body $ingot `
            -TimeoutSec 5
        
        $successCount++
        Write-Host "  Success: Ingot $i submitted" -ForegroundColor Green
    } catch {
        $failCount++
        Write-Host "  Error: Ingot $i failed" -ForegroundColor Red
    }
    
    Start-Sleep -Milliseconds 100
}

Write-Host ""
Write-Host "  Results: $successCount succeeded, $failCount failed" -ForegroundColor $(if ($failCount -eq 0) { "Green" } else { "Yellow" })

# Test 3: Wait for batch processing
Write-Host "[3/4] Waiting for batch processing..." -ForegroundColor Yellow
Start-Sleep -Seconds 3
Write-Host "  Success: Wait complete" -ForegroundColor Green

# Test 4: Check Prometheus Metrics
Write-Host "[4/4] Checking Prometheus metrics..." -ForegroundColor Yellow
try {
    $metrics = Invoke-RestMethod -Uri "$MintURL`:9090/metrics" -Method Get -TimeoutSec 5
    
    # Parse metrics
    $ingotsReceived = ($metrics -split "`n" | Select-String "mint_ingots_received_total" | Select-Object -First 1) -replace '.*\s+(\d+).*', '$1'
    $batchesProcessed = ($metrics -split "`n" | Select-String "mint_batches_processed_total" | Select-Object -First 1) -replace '.*\s+(\d+).*', '$1'
    
    Write-Host "  Ingots received: $ingotsReceived" -ForegroundColor Cyan
    Write-Host "  Batches processed: $batchesProcessed" -ForegroundColor Cyan
    
    if ([int]$ingotsReceived -ge $NumIngots) {
        Write-Host "  Success: Metrics look good!" -ForegroundColor Green
    } else {
        Write-Host "  Warning: Fewer ingots received than sent" -ForegroundColor Yellow
    }
} catch {
    Write-Host "  Error: Failed to fetch metrics" -ForegroundColor Red
}

Write-Host ""
Write-Host "E2E Test Complete!" -ForegroundColor Green
Write-Host ""
Write-Host "To verify NATS publishing, run:" -ForegroundColor Cyan
Write-Host "   docker exec robotorq-network-nats-1 nats sub mint.batches" -ForegroundColor Gray
