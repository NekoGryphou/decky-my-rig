# - Compiles the Windows installer and runs its procedures in owned fixtures.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).ProviderPath
$packaging = Join-Path $root 'packaging\windows'
$compilerOutput = @(& (Join-Path $packaging 'get_inno_setup.ps1'))
if ($compilerOutput.Count -ne 1) { throw "get_inno_setup.ps1 must return only the compiler path, got $($compilerOutput.Count) values" }
$compiler = $compilerOutput[0]
$failures = New-Object System.Collections.Generic.List[string]

function Assert-True([bool] $condition, [string] $message) {
    if (-not $condition) { throw $message }
}

function New-OwnedDirectory {
    $path = Join-Path $root ("out\installer-test-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $path | Out-Null
    $path
}

function Invoke-Compiler([string] $script, [string] $output, [hashtable] $defines) {
    $arguments = @('--quiet-progress', "--output-dir=$output")
    foreach ($name in $defines.Keys) { $arguments += "--define=$name=$($defines[$name])" }
    $arguments += $script
    $log = & $compiler @arguments 2>&1 | Out-String
    Assert-True ($LASTEXITCODE -eq 0) "ISCC failed for ${script}: $log"
}

function New-Fixture([string] $name, [string] $base, [hashtable] $defines = @{}) {
    $defines['CodeDir'] = Join-Path $packaging 'code'
    Invoke-Compiler (Join-Path $PSScriptRoot "${name}_fixtures.iss") $base $defines
    Join-Path $base "${name}_fixture.exe"
}

# * A fixture's InitializeSetup writes its result and returns False (exit 1).
function Invoke-Fixture([string] $fixture, [string[]] $arguments) {
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.UseShellExecute = $false
    $info.FileName = $fixture
    $info.Arguments = (@('/SP-', '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') + $arguments |
        ForEach-Object { '"' + $_ + '"' }) -join ' '
    $process = [Diagnostics.Process]::Start($info)
    try { $process.WaitForExit(); $process.ExitCode } finally { $process.Dispose() }
}

function Test-FirewallRuleCoversEveryProfileOnTheLocalSubnet {
    # ! A network left on Public, or one Windows misclassifies, must not silently block the device;
    # ! LocalSubnet keeps the rule off the internet. The fixture applies the installer's real
    # ! settings to a scratch rule and reads them back through the firewall API.
    $base = New-OwnedDirectory
    try {
        $fixture = New-Fixture 'firewall' $base
        $result = Join-Path $base 'result.txt'
        Assert-True ((Invoke-Fixture $fixture @("/ResultFile=$result")) -eq 1) 'fixture exit'
        $summary = [IO.File]::ReadAllText($result)
        $expected = 'profiles=2147483647;scope=LocalSubnet;action=1;direction=1;protocol=6;ports=*;enabled=yes;app=C:\MyRig\myrig-service.exe'
        Assert-True ($summary -eq $expected) "firewall rule: $summary"
        Assert-True (-not (Get-NetFirewallRule -Name 'MyRig-test-firewall' -ErrorAction SilentlyContinue)) 'scratch rule removed'
    } finally { Remove-Item -LiteralPath $base -Recurse -Force }
}

function Test-ProtectingAFolderNeverRestoresInheritedRights {
    # ! /reset restores the parent's inherited ACEs, and ProgramData lets Users create subfolders.
    # ! Removing inheritance afterwards is too late for that window, so a protected folder must
    # ! never pass through /reset.
    $body = [regex]::Match((Get-Content -Raw (Join-Path $packaging 'code\storage.iss')),
        '(?s)procedure ProtectDirectory\(.*?\nend;').Value
    Assert-True ($body.Length -gt 0) 'ProtectDirectory found'
    Assert-True (-not $body.Contains("' /reset'")) 'no icacls /reset before the folder is protected'
    Assert-True ($body.IndexOf('/inheritance:r') -ge 0) 'inheritance removed'
}

function Test-InstallerCompiles {
    $base = New-OwnedDirectory
    try {
        $payload = Join-Path $base 'payload'
        New-Item -ItemType Directory -Path (Join-Path $payload 'App') | Out-Null
        foreach ($name in 'myrig-service.exe', 'myrig-setup.exe', 'myrig-notifier.exe',
            'myrig-host.exe', 'App\MyRigConfig.exe') {
            Set-Content -LiteralPath (Join-Path $payload $name) -Value 'owned package fixture'
        }
        $output = Join-Path $base 'result'
        New-Item -ItemType Directory -Path $output | Out-Null
        Invoke-Compiler (Join-Path $packaging 'my_rig.iss') $output @{
            PayloadDir = $payload; AppVersion = '0.1.0'
        }
        Assert-True (@(Get-ChildItem -LiteralPath $output -Filter '*.exe').Count -eq 1) 'one installer'
    } finally { Remove-Item -LiteralPath $base -Recurse -Force }
}

function Test-InstallerCompilesForEachArchitecture {
    $base = New-OwnedDirectory
    try {
        $payload = Join-Path $base 'payload'
        New-Item -ItemType Directory -Path (Join-Path $payload 'App') | Out-Null
        foreach ($name in 'myrig-service.exe', 'myrig-setup.exe', 'myrig-notifier.exe',
            'myrig-host.exe', 'App\MyRigConfig.exe') {
            Set-Content -LiteralPath (Join-Path $payload $name) -Value 'owned package fixture'
        }
        $output = Join-Path $base 'result'
        New-Item -ItemType Directory -Path $output | Out-Null
        Invoke-Compiler (Join-Path $packaging 'my_rig.iss') $output @{
            PayloadDir = $payload; AppVersion = '0.1.0'; Arch = 'aarch64'
        }
        Assert-True (Test-Path -LiteralPath (Join-Path $output 'myrig-app-v0.1.0-aarch64-windows.exe')) 'aarch64 installer name'
        $refused = $false
        try {
            Invoke-Compiler (Join-Path $packaging 'my_rig.iss') $output @{
                PayloadDir = $payload; AppVersion = '0.1.0'; Arch = 'riscv64'
            }
        } catch { $refused = $true }
        Assert-True $refused 'an unknown Arch must not compile'
    } finally { Remove-Item -LiteralPath $base -Recurse -Force }
}

function Test-RecordedManagersAreKeptOnceEach {
    $base = New-OwnedDirectory
    try {
        $fixture = New-Fixture 'manager_users' $base
        $result = Join-Path $base 'users.txt'
        $key = 'MyRig-ManagerUsers-' + [guid]::NewGuid().ToString('N')
        Assert-True ((Invoke-Fixture $fixture @("/FixtureKey=$key", "/ResultFile=$result")) -eq 1) 'fixture exit'
        $users = @([IO.File]::ReadAllLines($result) | Where-Object { $_ })
        Assert-True (($users -join ',') -eq 'S-1-5-21-100-200-300-1001,S-1-5-21-100-200-300-1002') "recorded: $($users -join ',')"
        Assert-True (-not (Test-Path "HKCU:\Software\$key")) 'the fixture key is removed'
    } finally { Remove-Item -LiteralPath $base -Recurse -Force }
}

function Test-InstallationDirectoryAdmission {
    $helper = Join-Path $root 'out\cargo\x86_64-pc-windows-msvc\release\myrig-setup.exe'
    Assert-True (Test-Path -LiteralPath $helper -PathType Leaf) 'run tools/windows/check_windows.ps1 first'
    $base = New-OwnedDirectory
    try {
        $fixture = New-Fixture 'directory' $base @{ HelperExe = $helper }
        $safe = Join-Path $env:ProgramFiles 'MyRig-directory-fixture-new'
        $cases = @(
            @($safe, '', $true),
            @($env:TEMP, '', $false),
            @($safe, (Join-Path $env:ProgramFiles 'Other-installed-folder'), $false)
        )
        for ($index = 0; $index -lt $cases.Count; $index++) {
            $path, $existing, $accepted = $cases[$index]
            $result = Join-Path $base "result-$index.txt"
            $code = Invoke-Fixture $fixture @("/TestPath=$path", "/ExistingApp=$existing", "/ResultFile=$result")
            Assert-True ($code -eq 1) "fixture exit $code for $path"
            $text = [IO.File]::ReadAllText($result)
            Assert-True (($text -eq '') -eq $accepted) "admission of '$path' (existing '$existing'): '$text'"
        }
    } finally { Remove-Item -LiteralPath $base -Recurse -Force }
}

function Test-ManagerRightsSurviveWindowsSddlNormalization {
    $base = New-OwnedDirectory
    try {
        $fixture = New-Fixture 'service_acl' $base
        $result = Join-Path $base 'writes.txt'
        Assert-True ((Invoke-Fixture $fixture @("/ResultFile=$result")) -eq 1) 'fixture exit'
        Assert-True ([IO.File]::ReadAllText($result) -eq '1') 'a normalized descriptor must not get another equivalent ACE'
    } finally { Remove-Item -LiteralPath $base -Recurse -Force }
}

function Test-AppFilesGrantUsersAndServiceOnlyReadExecute {
    $base = New-OwnedDirectory
    $owned = Join-Path $base 'app'
    $owner = [Security.Principal.WindowsIdentity]::GetCurrent().User
    New-Item -ItemType Directory -Path $owned | Out-Null
    try {
        $file = Join-Path $owned 'owned.exe'
        Set-Content -LiteralPath $file -Value 'owned fixture'
        foreach ($path in $owned, $file) {
            $acl = Get-Acl -LiteralPath $path
            $acl.SetAccessRuleProtection($true, $false)
            $acl.SetAccessRule((New-Object Security.AccessControl.FileSystemAccessRule($owner, 'FullControl', 'Allow')))
            Set-Acl -LiteralPath $path -AclObject $acl
        }
        $fixture = New-Fixture 'app_access' $base
        $result = Join-Path $base 'result.txt'
        Assert-True ((Invoke-Fixture $fixture @("/OwnedDirectory=$owned", "/ResultFile=$result")) -eq 1) 'fixture exit'
        Assert-True ([IO.File]::ReadAllText($result) -eq 'done') 'fixture result'
        foreach ($path in $owned, $file) {
            # ! LocalService and Users: read and execute, nothing that writes.
            foreach ($sid in 'S-1-5-19', 'S-1-5-32-545') {
                $mask = 0
                foreach ($rule in (Get-Acl -LiteralPath $path).Access) {
                    $ruleSid = $rule.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value
                    if ($rule.AccessControlType -eq 'Allow' -and $ruleSid -eq $sid) {
                        $mask = $mask -bor [int]$rule.FileSystemRights
                    }
                }
                Assert-True ((($mask -band 0x200a9) -eq 0x200a9) -and (($mask -band 0xd0156) -eq 0)) "$sid on ${path}: $mask"
            }
        }
    } finally {
        Remove-Item -LiteralPath $base -Recurse -Force
    }
}

function Test-PublishedAppKeepsItsResources {
    # ! Without MSIX tooling the publish drops MyRigConfig.pri and the app crashes at start
    # ! (compiled XAML lives in the .pri).
    $project = Get-Content -Raw (Join-Path $root 'apps\windows-app\src\MyRigConfig.App\MyRigConfig.App.csproj')
    Assert-True $project.Contains('<EnableMsixTooling>true</EnableMsixTooling>') 'MSIX tooling'
    $builder = Get-Content -Raw (Join-Path $packaging 'build_installer.ps1')
    Assert-True $builder.Contains("'MyRigConfig.pri'") 'pri staged'
    Assert-True $builder.Contains('Assets\AppIcon.ico') 'icon staged'
}

$tests = @(
    'Test-FirewallRuleCoversEveryProfileOnTheLocalSubnet'
    'Test-ProtectingAFolderNeverRestoresInheritedRights'
    'Test-InstallerCompiles'
    'Test-InstallerCompilesForEachArchitecture'
    'Test-RecordedManagersAreKeptOnceEach'
    'Test-InstallationDirectoryAdmission'
    'Test-ManagerRightsSurviveWindowsSddlNormalization'
    'Test-AppFilesGrantUsersAndServiceOnlyReadExecute'
    'Test-PublishedAppKeepsItsResources'
)
foreach ($test in $tests) {
    try { & $test; Write-Host "ok   $test" }
    catch { $failures.Add("${test}: $_"); Write-Host "FAIL ${test}: $_" }
}
if ($failures.Count -gt 0) { throw "$($failures.Count) of $($tests.Count) installer tests failed." }
Write-Host "$($tests.Count) installer tests passed."
