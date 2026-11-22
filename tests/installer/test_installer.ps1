# - Compiles the Windows installer and runs its procedures in owned fixtures.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).ProviderPath
$packaging = Join-Path $root 'packaging\windows'
$compiler = (& (Join-Path $packaging 'get_inno_setup.ps1') | Select-Object -Last 1)
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
    # ! A network left on Public, or one Windows misclassifies, must not
    #   silently block the device; LocalSubnet keeps the rule off the internet.
    $code = Get-Content -Raw (Join-Path $packaging 'code\firewall.iss')
    $consts = Get-Content -Raw (Join-Path $packaging 'code\install_consts.iss')
    Assert-True $code.Contains('Rule.Profiles := FirewallAllProfiles;') 'firewall profiles'
    Assert-True $consts.Contains('FirewallAllProfiles = $7FFFFFFF;') 'all-profiles constant'
    Assert-True $code.Contains("Rule.RemoteAddresses := 'LocalSubnet';") 'LocalSubnet'
}

function Test-UpgradeReplacesTheWholeApp {
    # ? The app folder is cleared before install, so a file the new version drops does not stay.
    $script = Get-Content -Raw (Join-Path $packaging 'my_rig.iss')
    Assert-True ($script -match '(?m)^\[InstallDelete\]\r?\nType: filesandordirs; Name: "\{app\}\\App"$') 'App folder cleared before install'
}

function Test-FreshDataIsBornProtectedAndOnlyRecordedDataIsTrusted {
    # ! A user who can create folders in ProgramData must not get a window to
    #   plant files, nor make a failed install's leftovers look owned.
    $script = Get-Content -Raw (Join-Path $packaging 'my_rig.iss')
    $storage = Get-Content -Raw (Join-Path $packaging 'code\storage.iss')
    $rollback = Get-Content -Raw (Join-Path $packaging 'code\rollback.iss')
    Assert-True ($script -match '(?s)if FreshInstall then CreateProtectedData;\s*EnsureProtectedData;') 'created protected before use'
    Assert-True $storage.Contains("RunSetup('create-data '") 'created by myrig-setup'
    $recorded = [regex]::Match($storage, '(?s)function DataRecorded: Boolean;.*?end;').Value
    Assert-True (-not $recorded.Contains('AppUninstallKey')) 'the uninstall key is no ownership record'
    Assert-True ($rollback -match '(?s)if not DataCreated then Exit;\s*if not DelTree\(ConfigDirectory') 'rollback removes only its own folders'
}

function Test-ServiceHoldsOnlyTheNeededPrivileges {
    # ! Callers connect at Identification level, which needs no impersonation
    #   privilege; holding one would turn a LAN-side bug into a path to SYSTEM.
    $code = Get-Content -Raw (Join-Path $packaging 'code\service.iss')
    Assert-True $code.Contains("' SeChangeNotifyPrivilege/SeShutdownPrivilege');") 'service privileges'
}

function Test-OwnersCannotRewriteTheDataAcl {
    # ! The service's rewrites leave host.json owned by its token's default
    #   owner, which other LocalService services share: an OWNER RIGHTS entry
    #   replaces the owner's implicit right to change the ACL.
    $code = Get-Content -Raw (Join-Path $packaging 'code\storage.iss')
    Assert-True $code.Contains("'*S-1-3-4:(OI)(CI)(RC) '") 'folders: owner may only read the ACL'
    Assert-True $code.Contains("'*S-1-3-4:(RC) '") 'files: owner may only read the ACL'
}

function Test-WindowsRestartsAFailedService {
    # ! A fatal control-pipe or listener error ends the service; without
    #   recovery actions the PC stays unreachable until the next boot.
    $code = Get-Content -Raw (Join-Path $packaging 'code\service.iss')
    Assert-True $code.Contains("'failure ' + ServiceName + ' reset= 86400 actions= restart/5000/restart/60000//'") 'recovery actions'
    Assert-True $code.Contains("'failureflag ' + ServiceName + ' 1'") 'recovery on error exits'
}

function Test-ServiceEventsAreReadable {
    # ? An unregistered source shows "the description cannot be found" in
    #   Event Viewer instead of the service's message.
    $script = Get-Content -Raw (Join-Path $packaging 'my_rig.iss')
    $key = 'Subkey: "SYSTEM\CurrentControlSet\Services\EventLog\Application\MyRigHost"'
    Assert-True $script.Contains($key + '; ValueType: expandsz; ValueName: "EventMessageFile"; ValueData: "%SystemRoot%\Microsoft.NET\Framework\v4.0.30319\EventLogMessages.dll"; Flags: uninsdeletekey') 'message file'
    Assert-True $script.Contains($key + '; ValueType: dword; ValueName: "TypesSupported"; ValueData: 7') 'event types'
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
    $owned = Join-Path $env:TEMP ('MyRig-app-access-' + [guid]::NewGuid().ToString('N'))
    $owner = [Security.Principal.WindowsIdentity]::GetCurrent().User
    New-Item -ItemType Directory -Path $owned | Out-Null
    $base = New-OwnedDirectory
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
        Remove-Item -LiteralPath $owned -Recurse -Force
    }
}

function Test-PublishedAppKeepsItsResources {
    # ! Without MSIX tooling the publish drops MyRigConfig.pri and the app
    #   crashes at start (compiled XAML lives in the .pri).
    $project = Get-Content -Raw (Join-Path $root 'apps\windows-app\src\MyRigConfig.App\MyRigConfig.App.csproj')
    Assert-True $project.Contains('<EnableMsixTooling>true</EnableMsixTooling>') 'MSIX tooling'
    $builder = Get-Content -Raw (Join-Path $packaging 'build_installer.ps1')
    Assert-True $builder.Contains("'MyRigConfig.pri'") 'pri staged'
    Assert-True $builder.Contains('Assets\AppIcon.ico') 'icon staged'
}

$tests = @(
    'Test-FirewallRuleCoversEveryProfileOnTheLocalSubnet'
    'Test-UpgradeReplacesTheWholeApp'
    'Test-FreshDataIsBornProtectedAndOnlyRecordedDataIsTrusted'
    'Test-ServiceHoldsOnlyTheNeededPrivileges'
    'Test-OwnersCannotRewriteTheDataAcl'
    'Test-WindowsRestartsAFailedService'
    'Test-ServiceEventsAreReadable'
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
