# - Installs, upgrades and uninstalls the built installer on this PC, then checks
#   the resulting state.
# ! Destructive: it changes services, firewall rules and ProgramData, so it runs only with -Confirm
# ! on a PC with no My Rig installed, from an elevated session with a signed-in user (the installer
# ! reads the signed-in account).
param(
    [Parameter(Mandatory)][string] $Installer,
    [switch] $Confirm,
    # ? Each installer run writes its Inno log here, so a failure is readable.
    [string] $LogDirectory = (Join-Path $env:TEMP 'MyRig-end-to-end')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not $Confirm) { throw 'This test changes the machine. Run it with -Confirm.' }
$installer = (Resolve-Path -LiteralPath $Installer).ProviderPath
$installDir = Join-Path $env:ProgramFiles 'My Rig'
$data = Join-Path $env:ProgramData 'MyRig'
$security = Join-Path $env:ProgramData 'MyRigSecurity'
$setup = Join-Path $installDir 'myrig-setup.exe'
$failures = New-Object System.Collections.Generic.List[string]
$taskName = 'MyRigEndToEndInstall'

function Assert-Check([bool] $condition, [string] $message) {
    if (-not $condition) { $script:failures.Add($message); Write-Host "FAIL $message" }
    else { Write-Host "ok   $message" }
}

$script:runs = 0
New-Item -ItemType Directory -Path $LogDirectory -Force | Out-Null

function Invoke-InSignedInSession([string] $program, [string] $arguments) {
    $script:runs++
    $arguments = "$arguments /LOG=`"$(Join-Path $LogDirectory "run-$($script:runs).log")`""
    # ? The installer reads the signed-in account, which a process started from an elevated remote
    # ? session does not have. A task that runs only in that session does.
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
    $action = New-ScheduledTaskAction -Execute $program -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive -RunLevel Highest
    Register-ScheduledTask -TaskName $taskName -Action $action -Principal $principal -Force | Out-Null
    Start-ScheduledTask -TaskName $taskName
    $deadline = (Get-Date).AddMinutes(15)
    do { Start-Sleep -Seconds 3; $task = Get-ScheduledTask -TaskName $taskName }
    while ($task.State -eq 'Running' -and (Get-Date) -lt $deadline)
    $result = (Get-ScheduledTaskInfo -TaskName $taskName).LastTaskResult
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
    $result
}

function Get-ServicePrivileges {
    (sc.exe qprivs MyRigHost | Out-String) -split "`r?`n" |
        Where-Object { $_ -match 'Privilege' } |
        ForEach-Object { ($_ -replace '^.*?(Se\w+Privilege).*$', '$1').Trim() }
}

function Assert-InstalledState([string] $label) {
    $service = Get-CimInstance Win32_Service -Filter "Name='MyRigHost'"
    Assert-Check ($null -ne $service -and $service.State -eq 'Running' -and $service.StartMode -eq 'Auto') "${label}: service running and automatic"

    $failure = (sc.exe qfailure MyRigHost | Out-String)
    Assert-Check ($failure -match 'RESTART -- Delay = 5000' -and $failure -match 'RESTART -- Delay = 60000') "${label}: service restarts after a fatal error"

    $privileges = @(Get-ServicePrivileges)
    Assert-Check (($privileges -join ',') -eq 'SeChangeNotifyPrivilege,SeShutdownPrivilege') "${label}: service holds only the needed privileges ($($privileges -join ','))"

    $source = Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Services\EventLog\Application\MyRigHost' -ErrorAction SilentlyContinue
    Assert-Check ($null -ne $source -and $source.TypesSupported -eq 7 -and $source.EventMessageFile -match 'EventLogMessages\.dll') "${label}: event source registered"

    $rule = Get-NetFirewallRule -DisplayName MyRig -ErrorAction SilentlyContinue
    $scope = if ($rule) { (Get-NetFirewallAddressFilter -AssociatedNetFirewallRule $rule).RemoteAddress } else { '' }
    $profiles = if ($rule) { (([string] $rule.Profile) -split ',\s*') | Sort-Object } else { @() }
    Assert-Check ($null -ne $rule -and [string] $rule.Enabled -eq 'True') "${label}: firewall rule is enabled"
    # ? Windows reports all three profiles as 'Any'.
    Assert-Check ((($profiles -join ',') -eq 'Any') -or (($profiles -join ',') -eq 'Domain,Private,Public')) "${label}: firewall rule covers every profile ($($profiles -join ','))"
    Assert-Check ($scope -eq 'LocalSubnet') "${label}: firewall rule is limited to the local subnet ($scope)"

    foreach ($folder in $data, $security) {
        Assert-Check ((Get-Acl -LiteralPath $folder).Owner -eq 'BUILTIN\Administrators') "${label}: $folder owned by administrators"
    }
    # ! The service's own writes leave files owned by LocalService; an OWNER RIGHTS entry is what
    # ! keeps that owner from rewriting the ACL.
    foreach ($folder in $data, $security) {
        $acl = (icacls $folder | Out-String)
        Assert-Check ($acl -match 'OWNER RIGHTS:\(OI\)\(CI\)\(RC\)') "${label}: $folder limits its owner to reading the ACL"
    }
    & $setup check-data $data $security 2>$null | Out-Null
    Assert-Check ($LASTEXITCODE -eq 0) "${label}: data folders pass the service's own check"
}

# ? The ownership record and the data folders must start absent: a kept install leaves both behind,
# ? and then the data is recorded as ours.
$leftovers = @(
    if (Get-Service MyRigHost -ErrorAction SilentlyContinue) { 'the service' }
    if (Test-Path $installDir) { 'the app folder' }
    if (Test-Path $data) { 'the data folder' }
    if (Test-Path $security) { 'the security folder' }
    if (Test-Path 'HKLM:\SOFTWARE\MyRig') { 'the ownership record' }
    if (Get-LocalGroup -Name 'My Rig Managers' -ErrorAction SilentlyContinue) { 'the managers group' }
)
if ($leftovers.Count -gt 0) {
    throw "Remove what is left of My Rig first: $($leftovers -join ', ')."
}

try {
    # ? An unrecorded data folder must stop the install, not be adopted.
    New-Item -ItemType Directory -Path $data -Force | Out-Null
    $code = Invoke-InSignedInSession $installer '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART'
    Assert-Check ($code -ne 0) 'an unrecorded data folder is refused'
    Remove-Item -LiteralPath $data -Recurse -Force

    $code = Invoke-InSignedInSession $installer '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART'
    Assert-Check ($code -eq 0) "fresh install exits 0 ($code)"
    if ($code -eq 0) {
        Assert-InstalledState 'fresh install'

        # ? The upgrade must replace the whole app folder, not keep a file the new version drops.
        $stray = Join-Path $installDir 'App\stray-end-to-end.txt'
        Set-Content -LiteralPath $stray -Value 'left by an earlier version'
        $code = Invoke-InSignedInSession $installer '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART'
        Assert-Check ($code -eq 0) "upgrade over existing data exits 0 ($code)"
        Assert-Check (-not (Test-Path $stray)) 'upgrade removes files the app folder no longer ships'
        Assert-InstalledState 'upgrade'

        $uninstall = Join-Path $installDir 'unins000.exe'
        $code = Invoke-InSignedInSession $uninstall '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART'
        Assert-Check ($code -eq 0) "uninstall exits 0 ($code)"
        Start-Sleep -Seconds 3
        Assert-Check (-not (Get-Service MyRigHost -ErrorAction SilentlyContinue)) 'uninstall removes the service'
        Assert-Check (-not (Get-NetFirewallRule -DisplayName MyRig -ErrorAction SilentlyContinue)) 'uninstall removes the firewall rule'
        Assert-Check (-not (Test-Path $installDir)) 'uninstall removes the app folder'
    }
} finally {
    # ? Runs even when a check throws, so a failed run does not leave My Rig behind for the next
    # ? one. Data and the ownership record go only with what this run created.
    if (Test-Path (Join-Path $installDir 'unins000.exe')) {
        Invoke-InSignedInSession (Join-Path $installDir 'unins000.exe') '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' | Out-Null
    }
    foreach ($folder in $data, $security) { if (Test-Path $folder) { Remove-Item -LiteralPath $folder -Recurse -Force } }
    Remove-Item 'HKLM:\SOFTWARE\MyRig' -Recurse -ErrorAction SilentlyContinue
    if (Get-LocalGroup -Name 'My Rig Managers' -ErrorAction SilentlyContinue) { Remove-LocalGroup -Name 'My Rig Managers' }
}

if ($failures.Count -gt 0) { throw "$($failures.Count) end-to-end checks failed." }
Write-Host 'End-to-end installer checks passed.'
