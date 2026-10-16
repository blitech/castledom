# Lance Castledom en local : signaling Matchbox + Trunk (WASM).
# Usage:
#   .\scripts\dev.ps1           # tout (multi + web)
#   .\scripts\dev.ps1 -Solo     # Trunk seul (solo / sans multi)
#   .\scripts\dev.ps1 -Desktop  # natif + signaling (pas de WASM)

param(
    [switch]$Solo,
    [switch]$Desktop
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

# Trunk 0.21 casse si NO_COLOR=1 (Cursor / CI)
Remove-Item Env:NO_COLOR -ErrorAction SilentlyContinue

$signalingProc = $null

function Stop-Signaling {
    if ($null -eq $signalingProc) { return }
    if (-not $signalingProc.HasExited) {
        # tue cargo + enfants (binaire signaling)
        taskkill /PID $signalingProc.Id /T /F 2>$null | Out-Null
    }
}

try {
    if (-not $Solo) {
        Write-Host ">> signaling  ws://127.0.0.1:3536/castledom_<CODE>" -ForegroundColor Cyan
        $signalingProc = Start-Process -FilePath "cargo" `
            -ArgumentList @("run", "-p", "signaling") `
            -WorkingDirectory $root `
            -PassThru `
            -NoNewWindow
        Start-Sleep -Seconds 1
    }

    if ($Desktop) {
        Write-Host ">> desktop    cargo run -p game_app --bin castledom" -ForegroundColor Cyan
        & cargo run -p game_app --bin castledom
    } else {
        # Libère le port 8080 si un trunk précédent tourne encore (os error 10048)
        Get-Process -Name trunk -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
        Write-Host ">> web        http://127.0.0.1:8080" -ForegroundColor Cyan
        & trunk serve --color never
    }
}
finally {
    Stop-Signaling
}
