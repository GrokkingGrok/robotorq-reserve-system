# RoboTorq Development - Firewall Setup
# Run this ONCE as Administrator: Right-click → "Run with PowerShell" → Allow

Write-Host "🔥 Setting up Windows Firewall rules for RoboTorq development..." -ForegroundColor Cyan

# Rule 1: Allow port 9000 (Digger HTTP server)
New-NetFirewallRule -DisplayName "RoboTorq - Digger HTTP (Port 9000)" `
    -Direction Inbound `
    -Protocol TCP `
    -LocalPort 9000 `
    -Action Allow `
    -Profile Private,Domain `
    -ErrorAction SilentlyContinue

Write-Host "✅ Port 9000 (Digger) allowed" -ForegroundColor Green

# Rule 2: Allow port 4222 (NATS)
New-NetFirewallRule -DisplayName "RoboTorq - NATS (Port 4222)" `
    -Direction Inbound `
    -Protocol TCP `
    -LocalPort 4222 `
    -Action Allow `
    -Profile Private,Domain `
    -ErrorAction SilentlyContinue

Write-Host "✅ Port 4222 (NATS) allowed" -ForegroundColor Green

# Rule 3: Allow port 8080 (Refinery)
New-NetFirewallRule -DisplayName "RoboTorq - Refinery (Port 8080)" `
    -Direction Inbound `
    -Protocol TCP `
    -LocalPort 8080 `
    -Action Allow `
    -Profile Private,Domain `
    -ErrorAction SilentlyContinue

Write-Host "✅ Port 8080 (Refinery) allowed" -ForegroundColor Green

# Rule 4: Allow port 8081 (Mint)
New-NetFirewallRule -DisplayName "RoboTorq - Mint (Port 8081)" `
    -Direction Inbound `
    -Protocol TCP `
    -LocalPort 8081 `
    -Action Allow `
    -Profile Private,Domain `
    -ErrorAction SilentlyContinue

Write-Host "✅ Port 8081 (Mint) allowed" -ForegroundColor Green

Write-Host ""
Write-Host "🎉 Firewall setup complete! You won't see popups anymore." -ForegroundColor Green
Write-Host ""
Write-Host "To verify rules:" -ForegroundColor Yellow
Write-Host "  Get-NetFirewallRule | Where-Object {`$_.DisplayName -like '*RoboTorq*'}" -ForegroundColor Gray
Write-Host ""

# Pause so you can see the results
Read-Host "Press Enter to close"
