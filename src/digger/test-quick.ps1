# Quick test script for new Digger

Write-Host "🚀 Starting Digger v0.2.0..." -ForegroundColor Cyan

# Start Digger in background
$digger = Start-Process -FilePath "C:\Users\Jon\Documents\Project-Asimov\robotorq-network\src\digger\target\debug\digger.exe" `
    -PassThru `
    -WindowStyle Hidden

Write-Host "⏳ Waiting 2 seconds for server to start..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

# Test endpoints
try {
    Write-Host ""
    Write-Host "Testing GET /" -ForegroundColor Cyan
    $response = Invoke-WebRequest -Uri "http://localhost:9000/" -UseBasicParsing
    Write-Host "✅ Status: $($response.StatusCode)" -ForegroundColor Green
    Write-Host "📄 Response: $($response.Content)" -ForegroundColor Gray
    
    Write-Host ""
    Write-Host "Testing GET /status" -ForegroundColor Cyan
    $response = Invoke-WebRequest -Uri "http://localhost:9000/status" -UseBasicParsing
    Write-Host "✅ Status: $($response.StatusCode)" -ForegroundColor Green
    Write-Host "📄 Response: $($response.Content)" -ForegroundColor Gray
    
    Write-Host ""
    Write-Host "🎉 All tests passed!" -ForegroundColor Green
    
} catch {
    Write-Host "❌ Error: $_" -ForegroundColor Red
} finally {
    Write-Host ""
    Write-Host "🛑 Stopping Digger..." -ForegroundColor Yellow
    Stop-Process -Id $digger.Id -Force
    Write-Host "✅ Digger stopped" -ForegroundColor Green
}
