$ErrorActionPreference = "Stop"
$Repository = if ($env:RATIONALE_REPOSITORY) { $env:RATIONALE_REPOSITORY } else { "Ragosorio/Rationale" }
$Version = if ($env:RATIONALE_VERSION) { $env:RATIONALE_VERSION.TrimStart('v') } else {
  ((Invoke-RestMethod "https://api.github.com/repos/$Repository/releases/latest").tag_name).TrimStart('v')
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
  $expected = (Get-Content (Join-Path $Tmp "$Archive.sha256")).Split()[0].ToLowerInvariant()
  $actual = (Get-FileHash (Join-Path $Tmp $Archive) -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($expected -ne $actual) { throw "checksum inválido para $Archive" }
  Expand-Archive (Join-Path $Tmp $Archive) -DestinationPath $Tmp -Force
  New-Item -ItemType Directory -Force $Prefix | Out-Null
  Copy-Item (Join-Path $Tmp "rationale-$Version-$Target/rationale.exe") (Join-Path $Prefix "rationale.exe") -Force
  Write-Host "Rationale $Version instalado en $Prefix/rationale.exe"
} finally {
  Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
