// - Stops the processes that run this install's copy of an executable. Another
//   program's copy with the same name is left alone.
procedure StopOwnedProcesses(ImageName, ExePath: String);
var
  Locator, Services, Processes, Instance: Variant;
  I: Integer;
begin
  Locator := CreateOleObject('WbemScripting.SWbemLocator');
  Locator.Security_.Privileges.AddAsString('SeDebugPrivilege');
  Services := Locator.ConnectServer('.', 'root\cimv2');
  Processes := Services.ExecQuery(
    'SELECT Handle, ExecutablePath FROM Win32_Process WHERE Name="' + ImageName + '"');
  for I := 0 to Processes.Count - 1 do begin
    Instance := Processes.ItemIndex(I);
    if VarIsNull(Instance.ExecutablePath) then RaiseException(NotifierPathUnknown);
    if (CompareText(Instance.ExecutablePath, ExePath) = 0) and
      (Instance.Terminate(0) <> 0) then
      RaiseException(NotifierNotStopped);
  end;
end;

procedure StopOwnedNotifiers;
begin
  StopOwnedProcesses('myrig-notifier.exe', ExpandConstant('{app}\myrig-notifier.exe'));
end;

// ! The window app keeps files open while it runs, and the program folder can't be removed until it
// ! stops.
procedure StopOwnedApp;
begin
  StopOwnedProcesses('MyRigConfig.exe', ExpandConstant('{app}\App\MyRigConfig.exe'));
end;

procedure RemoveNotifierAutostart;
var
  Profiles: TArrayOfString;
  I: Integer;
  RunKey, ApprovedKey, IdentityKey, Value, NotifierPath: String;
begin
  NotifierPath := Quoted(ExpandConstant('{app}\myrig-notifier.exe'));
  if not RegGetSubkeyNames(HKEY_USERS, '', Profiles) then
    RaiseException(LoginEntriesUnreadable);
  for I := 0 to GetArrayLength(Profiles) - 1 do begin
    RunKey := Profiles[I] + '\Software\Microsoft\Windows\CurrentVersion\Run';
    ApprovedKey := Profiles[I] +
      '\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run';
    // ? auto-launch stores the quoted path followed by a space, and records the entry's enabled
    // ? state under StartupApproved.
    if RegQueryStringValue(HKEY_USERS, RunKey, NotifierRunValue, Value) and
      (CompareText(Trim(Value), NotifierPath) = 0) then begin
      if not RegDeleteValue(HKEY_USERS, RunKey, NotifierRunValue) then
        RaiseException(LoginEntryNotRemoved);
      if RegValueExists(HKEY_USERS, ApprovedKey, NotifierRunValue) and
        not RegDeleteValue(HKEY_USERS, ApprovedKey, NotifierRunValue) then
        RaiseException(LoginEntryNotRemoved);
    end;
    // - The notifier registers its notification identity next to its login entry.
    IdentityKey := Profiles[I] + NotifierIdentityKey;
    if RegKeyExists(HKEY_USERS, IdentityKey) and
      not RegDeleteKeyIncludingSubkeys(HKEY_USERS, IdentityKey) then
      RaiseException(NotifierIdentityNotRemoved);
  end;
end;
