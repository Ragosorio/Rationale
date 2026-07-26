$ErrorActionPreference = "Stop"
$Prefix = if ($env:RATIONALE_INSTALL_DIR) { $env:RATIONALE_INSTALL_DIR } else { Join-Path $HOME ".local/bin" }
$Binary = Join-Path $Prefix "rationale.exe"
if (Test-Path $Binary) { Remove-Item $Binary; Write-Host "eliminado: $Binary" }
Write-Host "No se modificó ningún directorio .rationale/."
