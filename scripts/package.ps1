$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Version = if ($env:RATIONALE_VERSION) { $env:RATIONALE_VERSION } else {
  (Select-String -Path (Join-Path $Root "Cargo.toml") -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
}
$Version = $Version.TrimStart('v')
$Target = if ($env:TARGET) { $env:TARGET } else { "x86_64-pc-windows-msvc" }
$OutDir = if ($env:OUT_DIR) { $env:OUT_DIR } else { Join-Path $Root "target/packages" }

cargo build --release --locked --target $Target
$Binary = Join-Path $Root "target/$Target/release/rationale.exe"
if (!(Test-Path $Binary)) { throw "No se encontró el binario compilado: $Binary" }

$Stage = Join-Path ([System.IO.Path]::GetTempPath()) ("rationale-package-" + [guid]::NewGuid())
$Bundle = Join-Path $Stage "rationale-$Version-$Target"
New-Item -ItemType Directory -Force $Bundle | Out-Null
Copy-Item $Binary (Join-Path $Bundle "rationale.exe")
Copy-Item (Join-Path $Root "LICENSE"), (Join-Path $Root "README.md") $Bundle
New-Item -ItemType Directory -Force $OutDir | Out-Null
$Archive = Join-Path $OutDir "rationale-$Version-$Target.zip"
# Sin el comodín '*': Compress-Archive incluye la carpeta $Bundle como
# entrada de nivel superior del ZIP, así el archivo lleva el mismo prefijo
# rationale-<version>-<target>/ que el .tar.xz de Unix (package.sh) y que
# rationale-installer.ps1 espera al extraer.
Compress-Archive -Path $Bundle -DestinationPath $Archive -Force
Get-FileHash $Archive -Algorithm SHA256 | ForEach-Object {
  "{0}  {1}" -f $_.Hash.ToLowerInvariant(), (Split-Path $Archive -Leaf)
} | Set-Content "$Archive.sha256"
Remove-Item -Recurse -Force $Stage
Write-Host "creado: $Archive"
