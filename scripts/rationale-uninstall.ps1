$ErrorActionPreference = "Stop"
$Prefix = if ($env:RATIONALE_INSTALL_DIR) { $env:RATIONALE_INSTALL_DIR } else { Join-Path $HOME ".local/bin" }
$Binary = Join-Path $Prefix "rationale.exe"
if (Test-Path $Binary) {
  try {
    & $Binary uninstall-agent --global-only
  } catch {
    Write-Warning "no se pudieron revertir todos los registros globales de agentes: $_"
  }
  Remove-Item $Binary
  Write-Host "eliminado: $Binary"
}
$Updater = Join-Path $Prefix "rationale-update.ps1"
if (Test-Path $Updater) { Remove-Item $Updater }
Write-Host "No se modificó ningún directorio .rationale/."
