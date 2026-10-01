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
# "Continue" : PowerShell 5 traiterait sinon la sortie normale de git (stderr) comme une erreur fatale.
$ErrorActionPreference = "Continue"
function Git-Quiet { git @args 2>&1 | Out-Null }
Set-Location (Split-Path -Parent $PSScriptRoot)

git fetch origin $Branch
git checkout $Branch
git pull --ff-only origin $Branch
# Toujours : rapide si rien n'a changé, et indispensable quand une nouvelle dépendance a été ajoutée depuis la dernière fois.
npm install --no-audit --no-fund

# L'application tourne en tâche de fond ; la boucle ci-dessous tire les nouveaux commits.
$app = Start-Process -FilePath "npm.cmd" -ArgumentList "run", "tauri", "dev" -NoNewWindow -PassThru
Write-Host "Surveillance de origin/$Branch toutes les $IntervalSec s (Ctrl+C pour arrêter)." -ForegroundColor Cyan
try {
  while (-not $app.HasExited) {
    Start-Sleep -Seconds $IntervalSec
    $before = git rev-parse HEAD
    Git-Quiet fetch origin $Branch
    # Fast-forward uniquement : n'écrase jamais vos modifications locales (échoue sans rien casser).
    Git-Quiet pull --ff-only origin $Branch
    $after = git rev-parse HEAD
    if ($before -ne $after) {
      Write-Host ("[{0}] mis à jour : {1}" -f (Get-Date -Format HH:mm:ss), (git log -1 --format=%s)) -ForegroundColor Yellow
      if (git diff --name-only $before $after | Select-String -Pattern "package(-lock)?\.json") {
        Write-Host "Dépendances modifiées : installation automatique (si l'écran reste en erreur, fermez l'application et relancez le lanceur)." -ForegroundColor Yellow
        npm install --no-audit --no-fund | Out-Null
      }
    }
  }
} finally {
  if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue }
}
