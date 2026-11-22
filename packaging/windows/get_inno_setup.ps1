$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
# ? Windows PowerShell 5.1 may default to TLS 1.0, which nuget.org refuses, and
#   its progress bar makes the download several times slower.
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$ProgressPreference = 'SilentlyContinue'
$version = '7.1.0'
# ! NuGet's recorded package hash for Tools.InnoSetup 7.1.0.
$sha512 = 'FAD889AFB47147B136D64852B355DD7D063B123BFA31D8392F8C2D00B0A2E4549D33C1CE11D53CFEE32C838ABC2507A6242154786E04C4515BB446E04DCF0967'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).ProviderPath
$root = Join-Path $repoRoot "out\cache\inno-setup-$version"
$compiler = Join-Path $root 'tools\ISCC.exe'
if (-not (Test-Path -LiteralPath $compiler -PathType Leaf)) {
    $package = Join-Path $repoRoot "out\cache\tools.innosetup.$version.nupkg"
    New-Item -ItemType Directory -Path (Split-Path $package) -Force | Out-Null
    Invoke-WebRequest -UseBasicParsing -OutFile $package `
        -Uri "https://api.nuget.org/v3-flatcontainer/tools.innosetup/$version/tools.innosetup.$version.nupkg"
    if ((Get-FileHash -LiteralPath $package -Algorithm SHA512).Hash -ne $sha512) {
        Remove-Item -LiteralPath $package -Force
        throw "Inno Setup $version package does not match its pinned SHA-512."
    }
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::ExtractToDirectory($package, $root)
    Remove-Item -LiteralPath $package -Force
}
$compiler
