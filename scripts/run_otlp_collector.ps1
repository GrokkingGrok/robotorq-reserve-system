#!/usr/bin/env pwsh
# Simple helper to run the OTLP collector locally using the repo compose file.
Push-Location -Path (Split-Path -Parent $MyInvocation.MyCommand.Definition)
try {
    Write-Host "Starting OTLP collector via docker compose..."
    docker compose -f ..\ci\otlp-collector\docker-compose.yml up -d
    Write-Host "Collector started. Use 'docker logs otel-collector -f' to view logs."
} finally {
    Pop-Location
}
