# Quick test of headless Digger contract execution
# This script starts headless, sends a stake, and checks for ore delivery

$ErrorActionPreference = "Stop"

Write-Host "🧪 Quick Headless Test" -ForegroundColor Cyan
Write-Host ""

# Kill any existing headless process
Stop-Process -Name headless -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# Start headless Digger
Write-Host "🚀 Starting headless Digger..." -ForegroundColor Yellow
$headlessPath = ".\digger\src-tauri\target\debug\headless.exe"
$headlessProc = Start-Process -FilePath $headlessPath -PassThru -WindowStyle Hidden

# Wait for startup
Write-Host "⏳ Waiting for startup (3 seconds)..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# Test 1: Health check
Write-Host "`n📋 Test 1: Health check..." -ForegroundColor Cyan
try {
    $health = Invoke-RestMethod -Uri "http://localhost:9000/health" -Method Get
    Write-Host "✅ Health check passed: $health" -ForegroundColor Green
} catch {
    Write-Host "❌ Health check failed: $_" -ForegroundColor Red
    $headlessProc.Kill()
    exit 1
}

# Test 2: Get robot status
Write-Host "`n📋 Test 2: Robot status..." -ForegroundColor Cyan
try {
    $status = Invoke-RestMethod -Uri "http://localhost:9000/robot/status" -Method Get
    Write-Host "✅ Robot status:" -ForegroundColor Green
    Write-Host "   Digger ID: $($status.digger_id)"
    Write-Host "   Available: $($status.available)"
    Write-Host "   Contract: $($status.current_contract)"
} catch {
    Write-Host "❌ Robot status failed: $_" -ForegroundColor Red
    $headlessProc.Kill()
    exit 1
}

# Test 3: Send stake
Write-Host "`n📋 Test 3: Sending 10.5 RT stake..." -ForegroundColor Cyan
try {
    $stake = @{
        contract_id = "test-contract-headless"
        amount_rt = 10.5
    } | ConvertTo-Json
    
    $response = Invoke-RestMethod -Uri "http://localhost:9000/stake" -Method Post -Body $stake -ContentType "application/json"
    Write-Host "✅ Stake accepted:" -ForegroundColor Green
    Write-Host "   Status: $($response.status)"
    Write-Host "   Contract ID: $($response.contract_id)"
    Write-Host "   Message: $($response.message)"
} catch {
    Write-Host "❌ Stake request failed: $_" -ForegroundColor Red
    Write-Host "   Error Details: $($_.Exception.Message)" -ForegroundColor Red
    if ($_.Exception.Response) {
        $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
        $responseBody = $reader.ReadToEnd()
        Write-Host "   Response Body: $responseBody" -ForegroundColor Red
    }
    $headlessProc.Kill()
    exit 1
}

# Test 4: Wait for ore generation
Write-Host "`n📋 Test 4: Waiting for ore generation (15 seconds)..." -ForegroundColor Cyan
Write-Host "   Milestones should appear every 5 seconds" -ForegroundColor Gray
Start-Sleep -Seconds 15

# Test 5: Check Refinery logs
Write-Host "`n📋 Test 5: Checking Refinery for ore receipts..." -ForegroundColor Cyan
$oreLogs = docker logs robotorq-network-refinery-1 --since 20s 2>&1 | Select-String "ore" -CaseSensitive
if ($oreLogs) {
    Write-Host "✅ Ore delivery found in Refinery:" -ForegroundColor Green
    $oreLogs | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
} else {
    Write-Host "❌ No ore delivery logs found" -ForegroundColor Red
}

# Cleanup
Write-Host "`n*** Cleaning up..." -ForegroundColor Yellow
$headlessProc.Kill()

Write-Host "`n*** Test complete!" -ForegroundColor Green
