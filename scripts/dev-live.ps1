<#
  Lance l'application en mode dev et récupère automatiquement les nouveaux commits de la branche.
  - Changements React/CSS : rechargés à chaud dans la fenêtre (Vite HMR).
  - Changements Rust     : Tauri recompile et relance l'application tout seul.
  Usage (PowerShell, depuis la racine du dépôt) :  .\scripts\dev-live.ps1
  Ctrl+C pour arrêter.
#>
param(
  [string]$Branch = "claude/ecstatic-ritchie-flor84",
  [int]$IntervalSec = 10
)
$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)

git fetch origin $Branch
git checkout $Branch
git pull --ff-only origin $Branch
if (-not (Test-Path node_modules)) { npm install }

# L'application tourne en tâche de fond ; la boucle ci-dessous tire les nouveaux commits.
$app = Start-Process -FilePath "npm.cmd" -ArgumentList "run", "tauri", "dev" -NoNewWindow -PassThru
Write-Host "Surveillance de origin/$Branch toutes les $IntervalSec s (Ctrl+C pour arrêter)." -ForegroundColor Cyan
try {
  while (-not $app.HasExited) {
    Start-Sleep -Seconds $IntervalSec
    $before = git rev-parse HEAD
    git fetch origin $Branch 2>$null
    # Fast-forward uniquement : n'écrase jamais vos modifications locales (échoue sans rien casser).
    git pull --ff-only origin $Branch 2>$null | Out-Null
    $after = git rev-parse HEAD
    if ($before -ne $after) {
      Write-Host ("[{0}] mis à jour : {1}" -f (Get-Date -Format HH:mm:ss), (git log -1 --format=%s)) -ForegroundColor Yellow
      if (git diff --name-only $before $after | Select-String -Pattern "package(-lock)?\.json") {
        Write-Host "Dépendances modifiées : arrêtez (Ctrl+C) et relancez ce script." -ForegroundColor Red
        npm install | Out-Null
      }
    }
  }
} finally {
  if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue }
}
