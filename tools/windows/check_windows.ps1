$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'prerequisites.ps1')
Use-Rust
Assert-Prerequisites -Clippy
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).ProviderPath
$manifest = Join-Path $root 'Cargo.toml'
$app = Join-Path $root 'apps\windows-app\src\MyRigConfig.App\MyRigConfig.App.csproj'
$tests = Join-Path $root 'apps\windows-app\tests\MyRigConfig.Tests\MyRigConfig.Tests.csproj'
$env:CARGO_TARGET_DIR = Join-Path $root 'out\cargo'
# ! The service must not need a separately installed Visual C++ runtime.
$env:RUSTFLAGS = '-C target-feature=+crt-static'
# ? An explicit target keeps these builds apart from Linux ones in out\cargo.
$target = 'x86_64-pc-windows-msvc'

# * Clippy only checks, so the release profile's fat LTO would add nothing here.
Invoke-Checked cargo @('clippy', '--manifest-path', $manifest, '--locked', '--target', $target, '--workspace', '--all-features', '--all-targets', '--', '-D', 'warnings')
# * --all-features: the service half of host-windows is tested too.
Invoke-Checked cargo @('test', '--manifest-path', $manifest, '--locked', '--release', '--target', $target, '--all-features', '--lib', '-p', 'myrig-host-windows', '-p', 'myrig-client', '-p', 'myrig-client-windows')
# * The installer tests run the real setup helper.
Invoke-Checked cargo @('build', '--manifest-path', $manifest, '--locked', '--release', '--target', $target, '-p', 'myrig-setup')

Invoke-Checked dotnet @('restore', $app, '--locked-mode', '-p:Platform=x64', '-p:RuntimeIdentifier=win-x64')
Invoke-Checked dotnet @('restore', $tests, '--locked-mode')
Invoke-Checked dotnet @('test', $tests, '--no-restore', '--nologo')
Invoke-Checked dotnet @('build', $app, '--no-restore', '--nologo', '-p:Platform=x64', '-p:RuntimeIdentifier=win-x64', '-warnaserror')
# - Compiles the installer and runs its procedures in owned fixtures.
& (Join-Path $root 'tests\installer\test_installer.ps1')
