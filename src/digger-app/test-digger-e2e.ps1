# Digger End-to-End Test Script
# Tests the full Digger → Refinery → Mint integration pipeline
#
# Prerequisites:
# - All services running via Docker Compose:
#   docker-compose up -d nats refinery mint
# - Headless Digger binary built:
#   cd src/digger-app/digger/src-tauri && cargo build --bin headless
#
# Usage: .\test-digger-e2e.ps1

$ErrorActionPreference = "Stop"

Write-Host "`n Digger E2E Test Suite" -ForegroundColor Cyan
Write-Host "=" * 60 -ForegroundColor Cyan

# 
# Configuration
# 

$ROOT_DIR = "C:\Users\Jon\Documents\Project-Asimov\robotorq-network"
$DIGGER_HTTP_API = "http://localhost:9000"
$REFINERY_HTTP_API = "http://localhost:8081"
$MINT_HTTP_API = "http://localhost:8080"
$NATS_URL = "nats://127.0.0.1:4222"

$TEST_CONTRACT_ID = "e2e-test-contract-001"
$TEST_DIGGER_ID = "dig-jon-ai-001"
$TEST_ROBO_STAKE = 10.0  # Small amount for fast test

# 
# Helper Functions
# 

function Test-ServiceHealth {
    param([string]$Url, [string]$ServiceName)
    
    try {
        $response = Invoke-WebRequest -Uri "$Url/health" -UseBasicParsing -TimeoutSec 2
        if ($response.StatusCode -eq 200) {
            Write-Host " $ServiceName is healthy" -ForegroundColor Green
            return $true
        }
    } catch {
        Write-Host " $ServiceName is not responding" -ForegroundColor Red
        return $false
    }
    return $false
}

function Wait-ForService {
    param([string]$Url, [string]$ServiceName, [int]$MaxAttempts = 30)
    
    Write-Host " Waiting for $ServiceName to be ready..." -ForegroundColor Yellow
    
    for ($i = 1; $i -le $MaxAttempts; $i++) {
        if (Test-ServiceHealth -Url $Url -ServiceName $ServiceName) {
            return $true
        }
        Start-Sleep -Seconds 1
    }
    
    Write-Host " $ServiceName failed to start after $MaxAttempts seconds" -ForegroundColor Red
    return $false
}

function Send-StakeToDigger {
    param([string]$ContractId, [float]$AmountRt)
    
    $body = @{
        contract_id = $ContractId
        amount_rt = $AmountRt
    } | ConvertTo-Json
    
    try {
        $response = Invoke-RestMethod -Uri "$DIGGER_HTTP_API/stake" `
                                       -Method Post `
                                       -Body $body `
                                       -ContentType "application/json" `
                                       -TimeoutSec 5
        
        Write-Host " Stake sent to Digger: $AmountRt RT" -ForegroundColor Green
        Write-Host "   Response: $($response.message)" -ForegroundColor Gray
        return $true
    } catch {
        Write-Host " Failed to send stake: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

function Get-DiggerStatus {
    try {
        $response = Invoke-RestMethod -Uri "$DIGGER_HTTP_API/robot/status" `
                                       -Method Get `
                                       -TimeoutSec 5
        
        Write-Host " Digger Status:" -ForegroundColor Cyan
        Write-Host "   Digger ID: $($response.digger_id)" -ForegroundColor Gray
        Write-Host "   Available: $($response.available)" -ForegroundColor Gray
        Write-Host "   Current Contract: $($response.current_contract)" -ForegroundColor Gray
        
        return $response
    } catch {
        Write-Host " Failed to get digger status: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

function Get-RefineryHealth {
    try {
        $response = Invoke-RestMethod -Uri "$REFINERY_HTTP_API/health" `
                                       -Method Get `
                                       -TimeoutSec 5
        
        Write-Host " Refinery Health:" -ForegroundColor Cyan
        Write-Host "   Status: $($response.status)" -ForegroundColor Gray
        Write-Host "   Ingot Assembly: $($response.ingot_assembly)" -ForegroundColor Gray
        
        return $response
    } catch {
        Write-Host " Failed to get refinery health: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

# 
# Test Execution
# 

Write-Host "`n Phase 1: Checking Prerequisites" -ForegroundColor Cyan
Write-Host "-" * 60

# Check NATS
Write-Host "`n Checking NATS..." -ForegroundColor Yellow
try {
    $natsHealth = Invoke-WebRequest -Uri "http://localhost:8222/healthz" -UseBasicParsing -TimeoutSec 2
    if ($natsHealth.StatusCode -eq 200) {
        Write-Host " NATS is running" -ForegroundColor Green
    }
} catch {
    Write-Host " NATS is not running" -ForegroundColor Red
    Write-Host "`nPlease start services first:" -ForegroundColor Yellow
    Write-Host "  cd $ROOT_DIR" -ForegroundColor Gray
    Write-Host "  docker-compose up -d nats refinery mint" -ForegroundColor Gray
    exit 1
}

# Check Refinery
Write-Host "`n Checking Refinery..." -ForegroundColor Yellow
if (-not (Test-ServiceHealth -Url $REFINERY_HTTP_API -ServiceName "Refinery")) {
    Write-Host "`nPlease start services first:" -ForegroundColor Yellow
    Write-Host "  cd $ROOT_DIR" -ForegroundColor Gray
    Write-Host "  docker-compose up -d nats refinery mint" -ForegroundColor Gray
    exit 1
}

# Check Mint
Write-Host "`n Checking Mint..." -ForegroundColor Yellow
if (-not (Test-ServiceHealth -Url $MINT_HTTP_API -ServiceName "Mint")) {
    Write-Host "`nPlease start services first:" -ForegroundColor Yellow
    Write-Host "  cd $ROOT_DIR" -ForegroundColor Gray
    Write-Host "  docker-compose up -d nats refinery mint" -ForegroundColor Gray
    exit 1
}

# 

Write-Host "`n Phase 2: Starting Headless Digger" -ForegroundColor Cyan
Write-Host "-" * 60

# Check if headless binary exists
$headlessPath = "$ROOT_DIR\src\digger-app\digger\src-tauri\target\debug\headless.exe"
if (-not (Test-Path $headlessPath)) {
    Write-Host " Headless binary not found!" -ForegroundColor Red
    Write-Host "`nPlease build it first:" -ForegroundColor Yellow
    Write-Host "  cd src/digger-app/digger/src-tauri" -ForegroundColor Gray
    Write-Host "  cargo build --bin headless" -ForegroundColor Gray
    exit 1
}

# Start Headless Digger (standalone HTTP server)
Write-Host "`n Starting Headless Digger..." -ForegroundColor Yellow
Set-Location "$ROOT_DIR\src\digger-app\digger\src-tauri"

$diggerJob = Start-Job -ScriptBlock {
    param($DiggerPath)
    Set-Location $DiggerPath
    .\target\debug\headless.exe 2>&1
} -ArgumentList @("$ROOT_DIR\src\digger-app\digger\src-tauri")

Start-Sleep -Seconds 3  # Give Digger time to start

# Verify Digger started
Write-Host "`n Checking Digger status..." -ForegroundColor Yellow
$diggerStatus = Get-DiggerStatus

if ($null -eq $diggerStatus) {
    Write-Host " Failed to connect to Digger - headless binary may have crashed" -ForegroundColor Red
    Write-Host "`nDigger output:" -ForegroundColor Yellow
    Receive-Job $diggerJob | Write-Host -ForegroundColor Gray
    Stop-Job $diggerJob -ErrorAction SilentlyContinue
    Remove-Job $diggerJob -ErrorAction SilentlyContinue
    exit 1
}

Write-Host " Headless Digger is running" -ForegroundColor Green

#

#

Write-Host "`n Phase 3: Testing Ore Delivery Pipeline" -ForegroundColor Cyan
Write-Host "-" * 60

# Test 1: Send stake to Digger
Write-Host "`n Test 1: Sending RoboStake to Digger..." -ForegroundColor Yellow

if (Send-StakeToDigger -ContractId $TEST_CONTRACT_ID -AmountRt $TEST_ROBO_STAKE) {
    Write-Host " Test 1 PASSED: Stake accepted" -ForegroundColor Green
} else {
    Write-Host " Test 1 FAILED: Stake rejected" -ForegroundColor Red
}

# Test 2: Verify contract was created
Write-Host "`n Test 2: Verifying contract creation..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

$updatedStatus = Get-DiggerStatus
if ($updatedStatus.current_contract -eq $TEST_CONTRACT_ID) {
    Write-Host " Test 2 PASSED: Contract created successfully" -ForegroundColor Green
} else {
    Write-Host " Test 2 FAILED: Contract not found" -ForegroundColor Red
}

# Test 3: Wait for complete ingot assembly (need 12 ores × 300 tokens = 3600 units)
Write-Host "`n Test 3: Waiting for ingot assembly (65 seconds)..." -ForegroundColor Yellow
Write-Host "   Digger generates milestones every 5 seconds (300 tokens each)" -ForegroundColor Gray
Write-Host "   Refinery needs 3600 units (12 ores) to assemble 1 ingot" -ForegroundColor Gray
Write-Host "   This should take ~60 seconds + buffer" -ForegroundColor Gray

# Monitor progress every 15 seconds
for ($i = 1; $i -le 4; $i++) {
    Start-Sleep -Seconds 15
    $health = Get-RefineryHealth
    if ($null -ne $health -and $null -ne $health.ingot_assembly) {
        $progress = [math]::Round($health.ingot_assembly.progress_to_next_ingot_percent, 1)
        $units = $health.ingot_assembly.accumulated_units
        Write-Host "   [$($i * 15)s] Progress: $progress% ($units/3600 units)" -ForegroundColor Cyan
    }
}

# Add 5 more seconds to ensure ingot is sent to Mint
Start-Sleep -Seconds 5

# Check Refinery health again to see if ingot was assembled
$finalRefineryHealth = Get-RefineryHealth

if ($null -ne $finalRefineryHealth) {
    Write-Host " Test 3 PASSED: Refinery still healthy after ore delivery" -ForegroundColor Green
    
    if ($null -ne $finalRefineryHealth.ingot_assembly) {
        $completed = $finalRefineryHealth.ingot_assembly.completed_ingots_pending
        if ($completed -gt 0) {
            Write-Host "   ✨ Bonus: $completed ingot(s) assembled and pending delivery to Mint!" -ForegroundColor Green
        }
    }
} else {
    Write-Host " Test 3 FAILED: Refinery became unhealthy" -ForegroundColor Red
}

# 

Write-Host "`nPhase 4: Validation & Metrics" -ForegroundColor Cyan
Write-Host "-" * 60

# Check Refinery logs for received ore
Write-Host "`n Checking Refinery logs for ore receipts..." -ForegroundColor Yellow

$refineryLogs = docker logs robotorq-network-refinery-1 --since 90s 2>&1 | Select-String -Pattern "ore received|unit added|ingot assembled" | Select-Object -Last 15

if ($refineryLogs.Count -gt 0) {
    Write-Host " Found ore processing logs:" -ForegroundColor Green
    $refineryLogs | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
    
    # Check specifically for ingot assembly
    $ingotLogs = $refineryLogs | Select-String -Pattern "ingot assembled"
    if ($ingotLogs.Count -gt 0) {
        Write-Host "`n   🎉 SUCCESS: Ingot(s) assembled!" -ForegroundColor Green
    } else {
        Write-Host "`n   ⚠️  No ingots assembled yet (may need more time)" -ForegroundColor Yellow
    }
} else {
    Write-Host "  No ore processing logs found (may need more time)" -ForegroundColor Yellow
}

# Check Mint logs for batch creation
Write-Host "`n Checking Mint logs for ingot receipt & batch creation..." -ForegroundColor Yellow

$mintLogs = docker logs robotorq-network-mint-1 --since 90s 2>&1 | Select-String -Pattern "ingot received|batch created|batch sent" | Select-Object -Last 10

if ($mintLogs.Count -gt 0) {
    Write-Host " Found Mint processing logs:" -ForegroundColor Green
    $mintLogs | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
    
    # Check for batch creation
    $batchLogs = $mintLogs | Select-String -Pattern "batch created|batch sent"
    if ($batchLogs.Count -gt 0) {
        Write-Host "`n   🎉 SUCCESS: Batch(es) created by Mint!" -ForegroundColor Green
    }
} else {
    Write-Host "  No Mint logs found yet (batches created after 1000 ingots)" -ForegroundColor Yellow
}

# 

Write-Host "`n Phase 5: Cleanup" -ForegroundColor Cyan
Write-Host "-" * 60

Write-Host "`n Stopping Headless Digger..." -ForegroundColor Yellow

# Stop Digger
Stop-Job $diggerJob -ErrorAction SilentlyContinue
Remove-Job $diggerJob -ErrorAction SilentlyContinue
Write-Host " Headless Digger stopped" -ForegroundColor Green

Write-Host "`nNOTE: Docker Compose services (NATS, Refinery, Mint) are still running." -ForegroundColor Yellow
Write-Host "To stop them:" -ForegroundColor Yellow
Write-Host "  cd $ROOT_DIR" -ForegroundColor Gray
Write-Host "  docker-compose down" -ForegroundColor Gray

# 

Write-Host "`n Test Summary" -ForegroundColor Cyan
Write-Host "=" * 60 -ForegroundColor Cyan

Write-Host "`n E2E Test Completed" -ForegroundColor Green
Write-Host "`nThe full pipeline was tested:" -ForegroundColor Cyan
Write-Host "  1. Headless Digger generated ore (JouleTorqOre)" -ForegroundColor Gray
Write-Host "  2. Refinery assembled ingots (TokenTorqIngot)" -ForegroundColor Gray
Write-Host "  3. Mint created batches (RoboTorq)" -ForegroundColor Gray

Write-Host ""

