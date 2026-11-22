function RegisteredAppDirectory: String;
begin
  if not RegQueryStringValue(HKLM, AppUninstallKey, 'InstallLocation', Result) then
    Result := '';
  if Result <> '' then Result := RemoveBackslashUnlessRoot(ExpandFileName(Result));
end;

// Any user can create folders in ProgramData, so a marker file proves nothing alone.
// The data must also be recorded in HKLM, which only administrators can write.
// The uninstall key is not proof either: it can remain after the data is removed.
function DataRecorded: Boolean;
begin
  Result := RegValueExists(HKLM, OwnershipKey, DataOwnedValue);
end;

procedure VerifyOwnedDirectory(Root: String);
var
  Marker: AnsiString;
begin
  if not DirExists(Root) then Exit;
  if not DataRecorded then RaiseException(UnrecordedData + Root);
  if not LoadStringFromFile(Root + '\' + OwnerMarkerFile, Marker) or
    (Marker <> OwnerMarker) then
    RaiseException(ConfigurationNotOwned + Root);
end;

procedure VerifyOwnedConfiguration;
begin
  VerifyOwnedDirectory(ConfigDirectory);
  VerifyOwnedDirectory(SecurityDirectory);
end;

procedure ProtectDirectory(Path, ServiceRights: String);
begin
  if not ForceDirectories(Path) then RaiseException(ConfigurationDirectoryFailed);
  RunRequired(SystemTool('icacls.exe'), Quoted(Path) + ' /setowner *S-1-5-32-544');
  RunRequired(SystemTool('icacls.exe'), Quoted(Path) + ' /reset');
  RunRequired(SystemTool('icacls.exe'), Quoted(Path) + ' /inheritance:r');
  RunRequired(SystemTool('icacls.exe'), Quoted(Path) +
    ' /grant:r *S-1-5-18:(OI)(CI)F *S-1-5-32-544:(OI)(CI)F ' + '*S-1-3-4:(OI)(CI)(RC) ' +
    Quoted(ServiceAccount + ':(OI)(CI)' + ServiceRights));
end;

procedure MarkOwned(Root: String);
begin
  if not SaveStringToFile(Root + '\' + OwnerMarkerFile, OwnerMarker, False) then
    RaiseException(ConfigurationOwnershipUnrecorded);
end;

procedure EnsureProtectedData;
begin
  ProtectDirectory(ConfigDirectory, 'M');
  ProtectDirectory(SecurityDirectory, 'RX');
  ProtectDirectory(SecurityDirectory + '\Security', 'RX');
  MarkOwned(ConfigDirectory);
  MarkOwned(SecurityDirectory);
  if not RegWriteDWordValue(HKLM, OwnershipKey, DataOwnedValue, 1) then
    RaiseException(ConfigurationOwnershipUnrecorded);
end;

// OWNER RIGHTS (S-1-3-4) limits an owner to reading the ACL. The service's token
// is the default owner of host.json, and other LocalService services share it.
// Re-protecting a file can leave it with an empty DACL, and taking ownership
// works whatever the DACL says. The grants below then restore access.
procedure ProtectFile(Path, ServiceRights: String);
begin
  RunRequired(SystemTool('takeown.exe'), '/F ' + Quoted(Path) + ' /A');
  RunRequired(SystemTool('icacls.exe'), Quoted(Path) + ' /reset');
  RunRequired(SystemTool('icacls.exe'), Quoted(Path) +
    ' /inheritance:r /grant:r *S-1-5-18:F *S-1-5-32-544:F ' + '*S-1-3-4:(RC) ' +
    Quoted(ServiceAccount + ':' + ServiceRights));
end;

// Runs myrig-setup with Arguments; a refusal raises setup's own message.
procedure RunSetup(Arguments: String);
var
  ExitCode, I: Integer;
  Output: TExecOutput;
  Message: String;
begin
  if ExecAndCaptureOutput(ExpandConstant('{app}\myrig-setup.exe'), Arguments,
    '', SW_HIDE, ewWaitUntilTerminated, ExitCode, Output) and (ExitCode = 0) then
    Exit;
  Message := '';
  for I := 0 to GetArrayLength(Output.StdErr) - 1 do
    Message := Trim(Message + ' ' + Output.StdErr[I]);
  if Message = '' then Message := SetupStepFailed + 'myrig-setup.exe (' + IntToStr(ExitCode) + ').';
  RaiseException(Message);
end;

// Fresh folders are created closed to everyone but SYSTEM and Administrators,
// so no user can plant files in them first. A folder that appeared after
// PrepareToInstall is refused, not adopted.
procedure CreateProtectedData;
begin
  RunSetup('create-data ' + Quoted(ConfigDirectory) + ' ' + Quoted(SecurityDirectory) +
    ' ' + Quoted(SecurityDirectory + '\Security'));
  DataCreated := True;
end;

procedure ProvisionIfNeeded;
var
  ConfigPath, SecurityPath: String;
begin
  ConfigPath := ConfigDirectory + '\host.json';
  SecurityPath := SecurityDirectory + '\Security\host-security.json';
  if FileExists(ConfigPath) <> FileExists(SecurityPath) then
    RaiseException(IdentityIncomplete);
  // An existing configuration is protected from administrators too; the
  // service validates it when it starts.
  if not FileExists(ConfigPath) then
    RunSetup('init ' + Quoted(ConfigPath) + ' ' + Quoted(SecurityPath) + ' ' + FirstPort);
  ProtectFile(ConfigPath, 'M');
  ProtectFile(SecurityPath, 'R');
end;
