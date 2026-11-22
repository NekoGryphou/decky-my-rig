# - Builds the Windows binaries and the installer for one architecture into
#   out\release.
param([ValidateSet('x86_64', 'aarch64')] [string] $Arch = 'x86_64')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).ProviderPath
. (Join-Path $repoRoot 'tools\windows\prerequisites.ps1')
Use-Rust
Assert-Prerequisites -Arm64:($Arch -eq 'aarch64')
$appProject = Join-Path $repoRoot 'apps\windows-app\src\MyRigConfig.App\MyRigConfig.App.csproj'
$env:CARGO_TARGET_DIR = Join-Path $repoRoot 'out\cargo'
# ! The service must not need a separately installed Visual C++ runtime.
$env:RUSTFLAGS = '-C target-feature=+crt-static'
$target = "$Arch-pc-windows-msvc"
# - .NET's names for the same processor.
$runtime, $platform = if ($Arch -eq 'aarch64') { 'win-arm64', 'ARM64' } else { 'win-x64', 'x64' }
$cargoBuild = @('build', '--manifest-path', (Join-Path $repoRoot 'Cargo.toml'), '--locked', '--release', '--target', $target)
Invoke-Checked cargo ($cargoBuild + @('-p', 'myrig-service'))
Invoke-Checked cargo ($cargoBuild + @('-p', 'myrig-notifier', '-p', 'myrig-setup', '-p', 'myrig-host'))
$HostBinaryDirectory = Join-Path $env:CARGO_TARGET_DIR "$target\release"
Invoke-Checked dotnet @('restore', $appProject, '--locked-mode', "-p:Platform=$platform", "-p:RuntimeIdentifier=$runtime")
$output = Join-Path $repoRoot 'out\release'
$stage = Join-Path $repoRoot "out\build\windows-installer-$Arch"
$payload = Join-Path $stage 'payload'
$appOutput = Join-Path $payload 'App'
$script = Join-Path $PSScriptRoot 'my_rig.iss'
$version = (Get-Content -LiteralPath (Join-Path $repoRoot 'VERSION') -Raw).Trim()

$binaries = @('myrig-service.exe', 'myrig-setup.exe', 'myrig-notifier.exe', 'myrig-host.exe')
if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
New-Item -ItemType Directory -Path $appOutput -Force | Out-Null
foreach ($name in $binaries) {
    Copy-Item -LiteralPath (Join-Path $HostBinaryDirectory $name) -Destination (Join-Path $payload $name)
}

& dotnet publish $appProject --no-restore --nologo -m:1 -c Release -r $runtime `
    "-p:Platform=$platform" -p:WindowsPackageType=None -p:WindowsAppSDKSelfContained=true `
    -p:SelfContained=true -o $appOutput
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath (Join-Path $appOutput 'MyRigConfig.exe'))) {
    throw 'The WinUI app publish failed.'
}
Get-ChildItem -LiteralPath $appOutput -Filter '*.pdb' -Recurse | Remove-Item
foreach ($resource in @('MyRigConfig.pri', 'Assets\AppIcon.ico')) {
    if (-not (Test-Path -LiteralPath (Join-Path $appOutput $resource) -PathType Leaf)) {
        throw "The published WinUI app is missing $resource."
    }
}
$compiler = & (Join-Path $PSScriptRoot 'get_inno_setup.ps1')
& $compiler '--quiet-progress' "--output-dir=$output" "--define=PayloadDir=$payload" "--define=AppVersion=$version" "--define=Arch=$Arch" $script
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compilation failed.' }
$installer = Join-Path $output "myrig-app-v$version-$Arch-windows.exe"
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
    throw 'The compiled installer is missing.'
}
Write-Output $installer
