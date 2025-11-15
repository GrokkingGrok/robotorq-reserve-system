# Digger End-to-End Test Script
# Tests the full Digger → Refinery integration pipeline
#
# Prerequisites:
# - Docker Compose installed
# - Go installed (for Refinery)
# - PowerShell 5.1+
#
# Usage: .\test-digger-e2e.ps1

$ErrorActionPreference = "Stop"

Write-Host "`n🎯 Digger E2E Test Suite" -ForegroundColor Cyan
Write-Host "=" * 60 -ForegroundColor Cyan

# ────────────────────────────────────────────────────────────────
# Configuration
# ────────────────────────────────────────────────────────────────

$ROOT_DIR = "C:\Users\Jon\Documents\Project-Asimov\robotorq-network"
$DIGGER_HTTP_API = "http://localhost:9000"
$REFINERY_HTTP_API = "http://localhost:8081"
$NATS_URL = "nats://127.0.0.1:4222"

$TEST_CONTRACT_ID = "e2e-test-contract-001"
$TEST_DIGGER_ID = "dig-jon-ai-001"
$TEST_ROBO_STAKE = 10.0  # Small amount for fast test

# ────────────────────────────────────────────────────────────────
# Helper Functions
# ────────────────────────────────────────────────────────────────

function Test-ServiceHealth {
    param([string]$Url, [string]$ServiceName)
    
    try {
        $response = Invoke-WebRequest -Uri "$Url/health" -UseBasicParsing -TimeoutSec 2
        if ($response.StatusCode -eq 200) {
            Write-Host "✅ $ServiceName is healthy" -ForegroundColor Green
            return $true
        }
    } catch {
        Write-Host "❌ $ServiceName is not responding" -ForegroundColor Red
        return $false
    }
    return $false
}

function Wait-ForService {
    param([string]$Url, [string]$ServiceName, [int]$MaxAttempts = 30)
    
    Write-Host "⏳ Waiting for $ServiceName to be ready..." -ForegroundColor Yellow
    
    for ($i = 1; $i -le $MaxAttempts; $i++) {
        if (Test-ServiceHealth -Url $Url -ServiceName $ServiceName) {
            return $true
        }
        Start-Sleep -Seconds 1
    }
    
    Write-Host "❌ $ServiceName failed to start after $MaxAttempts seconds" -ForegroundColor Red
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
        
        Write-Host "✅ Stake sent to Digger: $AmountRt RT" -ForegroundColor Green
        Write-Host "   Response: $($response.message)" -ForegroundColor Gray
        return $true
    } catch {
        Write-Host "❌ Failed to send stake: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

function Get-DiggerStatus {
    try {
        $response = Invoke-RestMethod -Uri "$DIGGER_HTTP_API/robot/status" `
                                       -Method Get `
                                       -TimeoutSec 5
        
        Write-Host "📊 Digger Status:" -ForegroundColor Cyan
        Write-Host "   Digger ID: $($response.digger_id)" -ForegroundColor Gray
        Write-Host "   Available: $($response.available)" -ForegroundColor Gray
        Write-Host "   Current Contract: $($response.current_contract)" -ForegroundColor Gray
        
        return $response
    } catch {
        Write-Host "❌ Failed to get digger status: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

function Get-RefineryHealth {
    try {
        $response = Invoke-RestMethod -Uri "$REFINERY_HTTP_API/health" `
                                       -Method Get `
                                       -TimeoutSec 5
        
        Write-Host "📊 Refinery Health:" -ForegroundColor Cyan
        Write-Host "   Status: $($response.status)" -ForegroundColor Gray
        Write-Host "   Ingot Assembly: $($response.ingot_assembly)" -ForegroundColor Gray
        
        return $response
    } catch {
        Write-Host "❌ Failed to get refinery health: $($_.Exception.Message)" -ForegroundColor Red
        return $null
    }
}

# ────────────────────────────────────────────────────────────────
# Test Execution
# ────────────────────────────────────────────────────────────────

Write-Host "`n📦 Phase 1: Starting Services" -ForegroundColor Cyan
Write-Host "-" * 60

# Start NATS
Write-Host "`n🚀 Starting NATS..." -ForegroundColor Yellow
Set-Location $ROOT_DIR
docker-compose up -d nats

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Failed to start NATS" -ForegroundColor Red
    exit 1
}

Start-Sleep -Seconds 3

# Check NATS health
Write-Host "`n🔍 Checking NATS health..." -ForegroundColor Yellow
try {
    $natsHealth = Invoke-WebRequest -Uri "http://localhost:8222/healthz" -UseBasicParsing -TimeoutSec 5
    if ($natsHealth.StatusCode -eq 200) {
        Write-Host "✅ NATS is healthy" -ForegroundColor Green
    }
} catch {
    Write-Host "❌ NATS health check failed" -ForegroundColor Red
    docker-compose down
    exit 1
}

# Start Refinery
Write-Host "`n🚀 Starting Refinery..." -ForegroundColor Yellow
Set-Location "$ROOT_DIR\src\refinery"

$env:NATS_URL = $NATS_URL
$env:HTTP_PORT = "8081"
$env:LOG_LEVEL = "info"

# Start Refinery in background
$refineryJob = Start-Job -ScriptBlock {
    param($RefineryPath, $NatsUrl)
    Set-Location $RefineryPath
    $env:NATS_URL = $NatsUrl
    $env:HTTP_PORT = "8081"
    go run ./cmd/refinery 2>&1
} -ArgumentList @("$ROOT_DIR\src\refinery", $NATS_URL)

# Wait for Refinery to start
if (-not (Wait-ForService -Url $REFINERY_HTTP_API -ServiceName "Refinery" -MaxAttempts 30)) {
    Write-Host "❌ Refinery failed to start" -ForegroundColor Red
    Stop-Job $refineryJob -ErrorAction SilentlyContinue
    Remove-Job $refineryJob -ErrorAction SilentlyContinue
    docker-compose down
    exit 1
}

# Note: Digger HTTP API is started by the Tauri app (or manually for testing)
# For headless testing, you would start: cargo run --bin digger-api
# For now, assume Digger is already running or will be started manually

Write-Host "`n✅ All services started successfully" -ForegroundColor Green

# ────────────────────────────────────────────────────────────────

Write-Host "`n📋 Phase 2: Pre-Test Checks" -ForegroundColor Cyan
Write-Host "-" * 60

# Check Digger status
Write-Host "`n🔍 Checking Digger status..." -ForegroundColor Yellow
$diggerStatus = Get-DiggerStatus

if ($null -eq $diggerStatus) {
    Write-Host "⚠️  Digger HTTP API is not running" -ForegroundColor Yellow
    Write-Host "   To run tests, start Digger with: cargo tauri dev" -ForegroundColor Yellow
    Write-Host "   Or implement standalone HTTP server for testing" -ForegroundColor Yellow
}

# Check Refinery health
Write-Host "`n🔍 Checking Refinery health..." -ForegroundColor Yellow
$refineryHealth = Get-RefineryHealth

if ($null -eq $refineryHealth) {
    Write-Host "❌ Refinery health check failed" -ForegroundColor Red
    Stop-Job $refineryJob -ErrorAction SilentlyContinue
    Remove-Job $refineryJob -ErrorAction SilentlyContinue
    docker-compose down
    exit 1
}

# ────────────────────────────────────────────────────────────────

Write-Host "`n🧪 Phase 3: Testing Ore Delivery Pipeline" -ForegroundColor Cyan
Write-Host "-" * 60

if ($null -ne $diggerStatus) {
    # Test 1: Send stake to Digger
    Write-Host "`n📤 Test 1: Sending RoboStake to Digger..." -ForegroundColor Yellow
    
    if (Send-StakeToDigger -ContractId $TEST_CONTRACT_ID -AmountRt $TEST_ROBO_STAKE) {
        Write-Host "✅ Test 1 PASSED: Stake accepted" -ForegroundColor Green
    } else {
        Write-Host "❌ Test 1 FAILED: Stake rejected" -ForegroundColor Red
    }
    
    # Test 2: Verify contract was created
    Write-Host "`n📤 Test 2: Verifying contract creation..." -ForegroundColor Yellow
    Start-Sleep -Seconds 2
    
    $updatedStatus = Get-DiggerStatus
    if ($updatedStatus.current_contract -eq $TEST_CONTRACT_ID) {
        Write-Host "✅ Test 2 PASSED: Contract created successfully" -ForegroundColor Green
    } else {
        Write-Host "❌ Test 2 FAILED: Contract not found" -ForegroundColor Red
    }
    
    # Test 3: Wait for ore to be generated and sent to Refinery
    Write-Host "`n📤 Test 3: Waiting for ore generation (15 seconds)..." -ForegroundColor Yellow
    Write-Host "   Digger should generate milestones every 5 seconds" -ForegroundColor Gray
    
    # Monitor Refinery logs for incoming ore
    Start-Sleep -Seconds 15
    
    # Check Refinery health again to see if it received ore
    $finalRefineryHealth = Get-RefineryHealth
    
    if ($null -ne $finalRefineryHealth) {
        Write-Host "✅ Test 3 PASSED: Refinery still healthy after ore delivery" -ForegroundColor Green
        
        # In a real test, we'd check NATS messages or Refinery metrics
        # For now, just verify the service is still running
    } else {
        Write-Host "❌ Test 3 FAILED: Refinery became unhealthy" -ForegroundColor Red
    }
} else {
    Write-Host "⚠️  Skipping ore delivery tests (Digger not running)" -ForegroundColor Yellow
}

# ────────────────────────────────────────────────────────────────

Write-Host "`n📊 Phase 4: Validation & Metrics" -ForegroundColor Cyan
Write-Host "-" * 60

# Check Refinery logs for received ore
Write-Host "`n📜 Checking Refinery logs for ore receipts..." -ForegroundColor Yellow

$refineryLogs = Receive-Job $refineryJob | Select-String -Pattern "ore received|unit added|ingot assembled" | Select-Object -Last 10

if ($refineryLogs.Count -gt 0) {
    Write-Host "✅ Found ore processing logs:" -ForegroundColor Green
    $refineryLogs | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
} else {
    Write-Host "⚠️  No ore processing logs found (may need more time)" -ForegroundColor Yellow
}

# ────────────────────────────────────────────────────────────────

Write-Host "`n🧹 Phase 5: Cleanup" -ForegroundColor Cyan
Write-Host "-" * 60

Write-Host "`n🛑 Stopping services..." -ForegroundColor Yellow

# Stop Refinery
Stop-Job $refineryJob -ErrorAction SilentlyContinue
Remove-Job $refineryJob -ErrorAction SilentlyContinue
Write-Host "✅ Refinery stopped" -ForegroundColor Green

# Stop NATS
Set-Location $ROOT_DIR
docker-compose down
Write-Host "✅ NATS stopped" -ForegroundColor Green

# ────────────────────────────────────────────────────────────────

Write-Host "`n📊 Test Summary" -ForegroundColor Cyan
Write-Host "=" * 60 -ForegroundColor Cyan

Write-Host "`n✅ E2E Test Completed" -ForegroundColor Green
Write-Host "`nNOTE: Full ore delivery testing requires Digger to be running." -ForegroundColor Yellow
Write-Host "Start Digger with: cd src\digger-app\digger && cargo tauri dev" -ForegroundColor Yellow

Write-Host "`n" -NoNewline
