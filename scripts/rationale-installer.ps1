$ErrorActionPreference = "Stop"
$Repository = if ($env:RATIONALE_REPOSITORY) { $env:RATIONALE_REPOSITORY } else { "Ragosorio/Rationale" }
$Version = if ($env:RATIONALE_VERSION -and $env:RATIONALE_VERSION -ne "latest") { $env:RATIONALE_VERSION.TrimStart('v') } else {
  $Channel = if ($env:RATIONALE_CHANNEL) { $env:RATIONALE_CHANNEL } else { "stable" }
  if ($Channel -eq "stable") {
    ((Invoke-RestMethod "https://api.github.com/repos/$Repository/releases/latest").tag_name).TrimStart('v')
  } elseif ($Channel -eq "preview") {
    $release = (Invoke-RestMethod "https://api.github.com/repos/$Repository/releases?per_page=100" | Where-Object { $_.prerelease } | Select-Object -First 1)
    if (-not $release) { throw "no se encontró una Release preview para $Repository" }
    $release.tag_name.TrimStart('v')
  } else {
    throw "RATIONALE_CHANNEL debe ser stable o preview"
  }
}
$Target = "x86_64-pc-windows-msvc"
$Prefix = if ($env:RATIONALE_INSTALL_DIR) { $env:RATIONALE_INSTALL_DIR } else { Join-Path $HOME ".local/bin" }
$Base = "https://github.com/$Repository/releases/download/v$Version"
$Archive = "rationale-$Version-$Target.zip"
$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("rationale-install-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $Tmp | Out-Null
try {
  Invoke-WebRequest "$Base/$Archive" -OutFile (Join-Path $Tmp $Archive)
  Invoke-WebRequest "$Base/$Archive.sha256" -OutFile (Join-Path $Tmp "$Archive.sha256")
  Invoke-WebRequest "$Base/rationale-update.ps1" -OutFile (Join-Path $Tmp "rationale-update.ps1")
  $expected = (Get-Content (Join-Path $Tmp "$Archive.sha256")).Split()[0].ToLowerInvariant()
  $actual = (Get-FileHash (Join-Path $Tmp $Archive) -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($expected -ne $actual) { throw "checksum inválido para $Archive" }
  Expand-Archive (Join-Path $Tmp $Archive) -DestinationPath $Tmp -Force
  New-Item -ItemType Directory -Force $Prefix | Out-Null
  Copy-Item (Join-Path $Tmp "rationale-$Version-$Target/rationale.exe") (Join-Path $Prefix "rationale.exe") -Force
  Copy-Item (Join-Path $Tmp "rationale-update.ps1") (Join-Path $Prefix "rationale-update.ps1") -Force
  Write-Host "Rationale $Version instalado en $Prefix/rationale.exe"
  if (-not [Console]::IsOutputRedirected -and $null -eq $env:NO_COLOR -and $null -eq $env:CI -and $env:RATIONALE_NO_MASCOT -ne "1") {
    Write-Host "      (\__/)"
    Write-Host "     ( ˶>ᴗ<˶)  ✨"
    Write-Host "    ╭/  R  \╮"
    Write-Host "  ━━┿━━━━━━━┿━━"
    Write-Host "Chestie dice: ¡ya está! La versión quedó verificada y lista para usar."
  }
} finally {
  Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
