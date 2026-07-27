$ErrorActionPreference = "Stop"
$Repository = if ($env:RATIONALE_REPOSITORY) { $env:RATIONALE_REPOSITORY } else { "Ragosorio/Rationale" }
$DefaultChannel = "preview"
$Channel = if ($env:RATIONALE_CHANNEL) { $env:RATIONALE_CHANNEL } else { $DefaultChannel }
$Version = if ($env:RATIONALE_VERSION) { $env:RATIONALE_VERSION } else { "" }
$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("rationale-update-" + [guid]::NewGuid() + ".ps1")
try {
  if ([string]::IsNullOrWhiteSpace($Version) -or $Version -eq "latest") {
    if ($Channel -eq "preview") {
      $release = (Invoke-RestMethod "https://api.github.com/repos/$Repository/releases?per_page=100" | Where-Object { $_.prerelease } | Select-Object -First 1)
      if (-not $release) { throw "no se encontró una Release preview para $Repository" }
      $Version = $release.tag_name
      $InstallerUrl = "https://github.com/$Repository/releases/download/$Version/rationale-installer.ps1"
    } elseif ($Channel -eq "stable") {
      $Version = "latest"
      $InstallerUrl = "https://github.com/$Repository/releases/latest/download/rationale-installer.ps1"
    } else {
      throw "RATIONALE_CHANNEL debe ser stable o preview"
    }
  } else {
    $InstallerUrl = "https://github.com/$Repository/releases/download/v$($Version.TrimStart('v'))/rationale-installer.ps1"
  }
  Invoke-WebRequest $InstallerUrl -OutFile $Tmp
  $env:RATIONALE_REPOSITORY = $Repository
  $env:RATIONALE_VERSION = $Version
  $env:RATIONALE_CHANNEL = $Channel
  & powershell -NoProfile -ExecutionPolicy Bypass -File $Tmp
  if ($LASTEXITCODE -ne 0) { throw "la actualización terminó con código $LASTEXITCODE" }
} finally {
  Remove-Item $Tmp -Force -ErrorAction SilentlyContinue
}
