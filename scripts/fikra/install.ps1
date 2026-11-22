# Fikra Code installer for Windows.
# Usage (PowerShell): irm https://fikraapi.co.ke/code/install.ps1 | iex
#
# Asset contract (the release workflow must produce exactly this):
#   GitHub release tag:  vX.Y.Z
#   Asset name:          fikra-package-<target>.zip
#   Archive root:        bin\fikra.exe (entrypoint) plus helper binaries and resources,
#                        in the same layout as upstream's codex package.
#   <target> is one of:  x86_64-pc-windows-msvc, aarch64-pc-windows-msvc
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Repo = if ($env:FIKRA_REPO) { $env:FIKRA_REPO } else { 'Fikra-API/fikra-code' }
$Version = if ($env:FIKRA_VERSION) { $env:FIKRA_VERSION } else { 'latest' }
$FikraHome = if ($env:FIKRA_HOME) { $env:FIKRA_HOME } else { Join-Path $env:USERPROFILE '.fikra' }
$BinDir = Join-Path $FikraHome 'bin'

$arch = $env:PROCESSOR_ARCHITECTURE
switch ($arch) {
  'AMD64' { $target = 'x86_64-pc-windows-msvc' }
  'ARM64' { $target = 'aarch64-pc-windows-msvc' }
  default { throw "Unsupported CPU architecture: $arch" }
}

if ($Version -eq 'latest') {
  $release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
  $Version = $release.tag_name
  if (-not $Version) { throw "Could not find the latest release of $Repo" }
}

$asset = "fikra-package-$target.zip"
$url = "https://github.com/$Repo/releases/download/$Version/$asset"
$dest = Join-Path $FikraHome "packages\$Version"
$tmp = Join-Path $FikraHome 'downloads'
New-Item -ItemType Directory -Force -Path $tmp, $BinDir | Out-Null
$zip = Join-Path $tmp $asset

Write-Host "Installing Fikra Code $Version ($target)"
$tries = 0
while ($true) {
  try { Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing; break }
  catch { $tries++; if ($tries -ge 3) { throw "Download failed: $url" }; Start-Sleep -Seconds 2 }
}

try {
  $expected = (Invoke-RestMethod "$url.sha256").ToString().Split(' ')[0].Trim()
  $actual = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLower()
  if ($expected -and $expected.ToLower() -ne $actual) {
    Remove-Item $zip -Force
    throw 'Checksum mismatch, download removed; run the installer again'
  }
} catch [System.Net.WebException] { }

$partial = "$dest.partial"
if (Test-Path $partial) { Remove-Item $partial -Recurse -Force }
Expand-Archive -Path $zip -DestinationPath $partial
if (-not (Test-Path (Join-Path $partial 'bin\fikra.exe'))) { throw 'Archive is missing bin\fikra.exe' }
if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
Move-Item $partial $dest
Remove-Item $zip -Force

# A small launcher in a stable folder, so PATH never needs to change between versions.
$launcher = Join-Path $BinDir 'fikra.cmd'
Set-Content -Path $launcher -Encoding ASCII -Value "@echo off`r`n`"$dest\bin\fikra.exe`" %*"

# Keep only the two newest versions, to save disk space on small machines.
Get-ChildItem (Join-Path $FikraHome 'packages') -Directory -Filter 'v*' |
  Sort-Object LastWriteTime -Descending | Select-Object -Skip 2 |
  ForEach-Object { Remove-Item $_.FullName -Recurse -Force }

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not ($userPath -split ';' | Where-Object { $_ -eq $BinDir })) {
  [Environment]::SetEnvironmentVariable('Path', "$BinDir;$userPath", 'User')
}
if (-not ($env:Path -split ';' | Where-Object { $_ -eq $BinDir })) {
  $env:Path = "$BinDir;$env:Path"
}
Write-Host "Installed: $launcher"

if ($env:FIKRA_NO_LAUNCH -ne '1') {
  Write-Host "Launching Fikra Code..."
  & "$dest\bin\fikra.exe"
} else {
  Write-Host "Next: run 'fikra' in a project. It signs you in through your browser."
}
