# - Builds the Windows release files into out\release.
param([ValidateSet('x86_64', 'aarch64')] [string[]] $Architectures = @('x86_64'))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'prerequisites.ps1')
Use-Rust
Assert-Prerequisites -Arm64:($Architectures -contains 'aarch64')
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).ProviderPath
$version = (Get-Content -LiteralPath (Join-Path $root 'VERSION') -Raw).Trim()
$release = Join-Path $root 'out\release'
$env:CARGO_TARGET_DIR = Join-Path $root 'out\cargo'
# ? No Visual C++ runtime to install beside myrig.exe.
$env:RUSTFLAGS = '-C target-feature=+crt-static'

foreach ($arch in $Architectures) {
    & (Join-Path $root 'packaging\windows\build_installer.ps1') -Arch $arch
    $target = "$arch-pc-windows-msvc"
    Invoke-Checked cargo @('build', '--manifest-path', (Join-Path $root 'Cargo.toml'), '--locked', '--release', '--target', $target, '-p', 'myrig-cli')
    $archive = Join-Path $release "myrig-cli-v$version-$arch-windows.zip"
    Compress-Archive -LiteralPath (Join-Path $env:CARGO_TARGET_DIR "$target\release\myrig.exe") -DestinationPath $archive -Force
    $archive
}
