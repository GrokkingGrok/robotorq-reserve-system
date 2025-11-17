# Setup Grafana with Prometheus Data Source and Phase 5 Dashboard
# Run this after: docker-compose up -d

$GRAFANA_URL = "http://localhost:3001"
$GRAFANA_USER = "admin"
$GRAFANA_PASS = "admin"

# Create base64 auth header
$base64AuthInfo = [Convert]::ToBase64String([Text.Encoding]::ASCII.GetBytes("${GRAFANA_USER}:${GRAFANA_PASS}"))
$headers = @{
    Authorization = "Basic $base64AuthInfo"
    "Content-Type" = "application/json"
}

Write-Host "`n🔧 Setting up Grafana..." -ForegroundColor Cyan
Write-Host "============================`n" -ForegroundColor Cyan

# Step 1: Wait for Grafana to be ready
Write-Host "⏳ Waiting for Grafana to start..." -ForegroundColor Yellow
$maxAttempts = 30
$attempt = 0
while ($attempt -lt $maxAttempts) {
    try {
        $response = Invoke-WebRequest -Uri "$GRAFANA_URL/api/health" -Method Get -UseBasicParsing -TimeoutSec 2
        if ($response.StatusCode -eq 200) {
            Write-Host "✅ Grafana is ready!`n" -ForegroundColor Green
            break
        }
    } catch {
        $attempt++
        Start-Sleep -Seconds 2
    }
}

if ($attempt -eq $maxAttempts) {
    Write-Host "❌ Grafana failed to start. Check: docker logs grafana" -ForegroundColor Red
    exit 1
}

# Step 2: Add Prometheus data source
Write-Host "📊 Adding Prometheus data source..." -ForegroundColor Cyan

$datasourceBody = @{
    name = "Prometheus"
    type = "prometheus"
    url = "http://prometheus:9090"
    access = "proxy"
    isDefault = $true
} | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri "$GRAFANA_URL/api/datasources" -Method Post -Headers $headers -Body $datasourceBody
    Write-Host "✅ Prometheus data source added (ID: $($response.id))`n" -ForegroundColor Green
} catch {
    if ($_.Exception.Response.StatusCode -eq 409) {
        Write-Host "⚠️  Prometheus data source already exists`n" -ForegroundColor Yellow
    } else {
        Write-Host "❌ Failed to add data source: $($_.Exception.Message)" -ForegroundColor Red
    }
}

# Step 3: Import Phase 5 Dashboard
Write-Host "📈 Importing Phase 5 Pipeline dashboard..." -ForegroundColor Cyan

$dashboardJson = Get-Content -Path "Grafana/phase5-pipeline-dashboard.json" -Raw
$dashboardBody = $dashboardJson | ConvertFrom-Json

try {
    $response = Invoke-RestMethod -Uri "$GRAFANA_URL/api/dashboards/db" -Method Post -Headers $headers -Body $dashboardJson
    Write-Host "✅ Phase 5 dashboard imported!`n" -ForegroundColor Green
    Write-Host "   Dashboard URL: $GRAFANA_URL$($response.url)" -ForegroundColor White
} catch {
    Write-Host "❌ Failed to import dashboard: $($_.Exception.Message)" -ForegroundColor Red
}

# Step 4: Import Torq Observability Dashboard (if exists)
if (Test-Path "Grafana/torq-observability-dashboard.json") {
    Write-Host "📈 Importing Torq Observability dashboard..." -ForegroundColor Cyan
    
    $torqDashboard = Get-Content -Path "Grafana/torq-observability-dashboard.json" -Raw
    
    try {
        $response = Invoke-RestMethod -Uri "$GRAFANA_URL/api/dashboards/db" -Method Post -Headers $headers -Body $torqDashboard
        Write-Host "✅ Torq Observability dashboard imported!`n" -ForegroundColor Green
        Write-Host "   Dashboard URL: $GRAFANA_URL$($response.url)" -ForegroundColor White
    } catch {
        Write-Host "⚠️  Torq dashboard import failed (may already exist)`n" -ForegroundColor Yellow
    }
}

# Summary
Write-Host "`n🎉 GRAFANA SETUP COMPLETE!" -ForegroundColor Green
Write-Host "============================`n" -ForegroundColor Green

Write-Host "📊 Access Grafana:" -ForegroundColor Cyan
Write-Host "   URL: $GRAFANA_URL" -ForegroundColor White
Write-Host "   Username: $GRAFANA_USER" -ForegroundColor White
Write-Host "   Password: $GRAFANA_PASS`n" -ForegroundColor White

Write-Host "Dashboards:" -ForegroundColor Cyan
Write-Host "   - Phase 5 Pipeline (metrics for hash -> merkle -> ingot -> mint)" -ForegroundColor White
Write-Host "   - Torq Observability (DistoDam + Trust metrics)`n" -ForegroundColor White

Write-Host "Sample Metrics to Explore:" -ForegroundColor Cyan
Write-Host "   - refinery_phase2_ingots_sent_total" -ForegroundColor Gray
Write-Host "   - mint_phase2_ingots_received_total" -ForegroundColor Gray
Write-Host "   - refinery_merkle_trees_built_total" -ForegroundColor Gray
Write-Host "   - refinery_falcon_signatures_verified_total`n" -ForegroundColor Gray
