#ifndef PayloadDir
  #error PayloadDir must point to the built Windows payload.
#endif
#ifndef AppVersion
  #error AppVersion must be the release version from VERSION.
#endif
#ifndef Arch
  #define Arch "x86_64"
#endif
; - Inno Setup's name for the processors the payload was built for.
#if Arch == "aarch64"
  #define InnoArch "arm64"
#elif Arch == "x86_64"
  #define InnoArch "x64compatible"
#else
  #error Arch must be x86_64 or aarch64.
#endif
#define AppGuid "1775DC67-5BF3-4F15-A91A-10EEFCB29AD2"

[Setup]
AppId={{{#AppGuid}}
AppName=My Rig
AppVersion={#AppVersion}
DefaultDirName={autopf}\My Rig
DisableDirPage=no
UsePreviousAppDir=yes
PrivilegesRequired=admin
ArchitecturesAllowed={#InnoArch}
ArchitecturesInstallIn64BitMode={#InnoArch}
OutputDir=..\..\out\release
OutputBaseFilename=myrig-app-v{#AppVersion}-{#Arch}-windows
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
RestartApplications=yes

[InstallDelete]
Type: filesandordirs; Name: "{app}\App"

[Files]
Source: "{#PayloadDir}\myrig-service.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\myrig-setup.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\myrig-host.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\App\*"; DestDir: "{app}\App"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#PayloadDir}\myrig-notifier.exe"; DestDir: "{app}"; Flags: ignoreversion; AfterInstall: FinalizeInstall

; ? The service's event source. EventLogMessages.dll maps every event ID to its first insertion
; ? string, which is the whole message.
[Registry]
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Services\EventLog\Application\MyRigHost"; ValueType: expandsz; ValueName: "EventMessageFile"; ValueData: "%SystemRoot%\Microsoft.NET\Framework\v4.0.30319\EventLogMessages.dll"; Flags: uninsdeletekey
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Services\EventLog\Application\MyRigHost"; ValueType: dword; ValueName: "TypesSupported"; ValueData: 7

[Icons]
Name: "{commonprograms}\My Rig"; Filename: "{app}\App\MyRigConfig.exe"

[Run]
; ? A fresh install turns the login entry on; the notifier itself never adds it, so a later
; ? --disable-autostart stays in force across upgrades.
Filename: "{app}\myrig-notifier.exe"; Parameters: "--enable-autostart"; Flags: runasoriginaluser runhidden waituntilterminated; Check: FreshInstallSucceeded
Filename: "{app}\myrig-notifier.exe"; Flags: runasoriginaluser runhidden nowait; Check: InstallSucceeded

[Code]
#include "code\install_consts.iss"
#include "code\install_messages.iss"

var
  InstallFailure: String;
  FreshInstall, DataCreated, GroupWasPresent, FirewallCreated, ServiceWasStopped: Boolean;


#include "code\install_native.iss"
#include "code\install_utils.iss"
#include "code\storage.iss"
#include "code\directory.iss"
#include "code\app_files.iss"
#include "code\managers.iss"
#include "code\service_rights.iss"
#include "code\service.iss"
#include "code\firewall.iss"
#include "code\notifier.iss"
#include "code\rollback.iss"

procedure InitializeWizard;
begin
  if RegisteredAppDirectory <> '' then begin
    WizardForm.DirEdit.ReadOnly := True;
    WizardForm.DirBrowseButton.Enabled := False;
  end;
end;

function NextButtonClick(CurPageID: Integer): Boolean;
var
  Failure: String;
begin
  Result := True;
  if CurPageID = wpSelectDir then begin
    Failure := CheckInstallationDirectory(WizardDirValue);
    if Failure <> '' then begin
      MsgBox(Failure, mbError, MB_OK);
      Result := False;
    end;
  end;
end;

function InstallSucceeded: Boolean;
begin
  Result := InstallFailure = '';
end;

function FreshInstallSucceeded: Boolean;
begin
  Result := InstallSucceeded and FreshInstall;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  try
    Result := CheckInstallationDirectory(WizardDirValue);
    if Result <> '' then Exit;
    VerifyOwnedService;
    VerifyOwnedConfiguration;
    ClearStaleOwnership;
    FreshInstall := not ServiceExists and not DirExists(ConfigDirectory) and
      not DirExists(SecurityDirectory);
    GroupWasPresent := RunSucceeded(SystemTool('net.exe'), 'localgroup ' + Quoted(ManagerGroup));
    ServiceWasStopped := ServiceExists;
    if ServiceWasStopped then StopServiceAndWait;
    if FileExists(ExpandConstant('{app}\myrig-notifier.exe')) then
      StopOwnedNotifiers;
    StopOwnedApp;
  except
    Result := GetExceptionMessage;
  end;
end;

procedure FinalizeInstall;
begin
  try
    GrantAppReadAccess(ExpandConstant('{app}'));
    EnsureManagerGroup;
    EnsureService;
    if FreshInstall then CreateProtectedData
    else VerifyOwnedFolderAccess;
    EnsureProtectedData;
    ProvisionIfNeeded;
    EnsureFirewall;
    RunRequired(SystemTool('sc.exe'), 'start ' + ServiceName);
    WaitForServiceRunning;
  except
    InstallFailure := GetExceptionMessage;
    Log('My Rig installation failed: ' + InstallFailure);
    if FreshInstall then RollbackFreshInstall else RestartStoppedService;
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if (CurPageID = wpFinished) and not InstallSucceeded then begin
    WizardForm.FinishedHeadingLabel.Caption := InstallFailedHeading;
    WizardForm.FinishedLabel.Caption := InstallFailure + #13#10 + InstallFailedHint;
  end;
end;

function GetCustomSetupExitCode: Integer;
begin
  if InstallSucceeded then Result := 0 else Result := 4;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Policy, Rule: Variant;
  RuleFound: Boolean;
begin
  if CurUninstallStep <> usUninstall then Exit;
  VerifyOwnedService;
  VerifyOwnedConfiguration;
  // - Refuse before changing anything if the rule name belongs to another program.
  Policy := CreateOleObject('HNetCfg.FwPolicy2');
  RuleFound := FindOwnedFirewallRule(Policy, Rule);
  if ServiceExists then begin
    StopServiceAndWait;
    RunRequired(SystemTool('sc.exe'), 'delete ' + ServiceName);
  end;
  if RuleFound then Policy.Rules.Remove(FirewallRuleName);
  RemoveManagerGroup;
  StopOwnedNotifiers;
  StopOwnedApp;
  RemoveNotifierAutostart;
  // - Silent uninstall keeps the configuration.
  if SuppressibleMsgBox(KeepConfigurationQuestion, mbConfirmation, MB_YESNO, IDYES) = IDNO then begin
    // ? The ownership record goes only with the data, or the next install would refuse the leftover
    // ? folder as not its own.
    if DelTree(ConfigDirectory, True, True, True) and
       DelTree(SecurityDirectory, True, True, True) then
      RegDeleteValue(HKLM, OwnershipKey, DataOwnedValue);
  end;
  RegDeleteKeyIfEmpty(HKLM, OwnershipKey);
end;
