function SessionText(SessionId, InfoClass: DWORD): String;
var
  Buffer: DWORD_PTR;
  Bytes: DWORD;
begin
  Buffer := 0;
  if not WTSQuerySessionInformationW(0, SessionId, InfoClass, Buffer, Bytes) then
    RaiseException(SignedInUserUnknown);
  try
    if (Buffer = 0) or (Bytes < 2) then
      RaiseException(SignedInUserUnavailable);
    Result := CastIntegerToString(Buffer);
  finally
    if Buffer <> 0 then WTSFreeMemory(Buffer);
  end;
end;

function InteractiveAccountName: String;
var
  SessionId: DWORD;
  UserName, DomainName: String;
begin
  if not ProcessIdToSessionId(GetCurrentProcessId, SessionId) then
    RaiseException(InstallerSessionUnknown);
  UserName := SessionText(SessionId, 5);
  DomainName := SessionText(SessionId, 7);
  if (UserName = '') or (DomainName = '') then
    RaiseException(InstallerSessionRequired);
  Result := DomainName + '\' + UserName;
end;

function AccountSidPointer(AccountName: String): DWORD_PTR;
var
  SidSize, DomainSize, SidUse: DWORD;
  DomainName: String;
begin
  SidSize := 68;
  DomainSize := 256;
  SetLength(DomainName, DomainSize);
  Result := LocalAlloc(0, SidSize);
  if Result = 0 then RaiseException(AccountSidUnavailable);
  if not LookupAccountNameW('', AccountName, Result, SidSize,
    DomainName, DomainSize, SidUse) then begin
    LocalFree(Result);
    RaiseException(AccountUnresolved + AccountName + '.');
  end;
end;

function AccountSidText(AccountName: String): String;
var
  Sid, SidText: DWORD_PTR;
begin
  Sid := AccountSidPointer(AccountName);
  try
    if not ConvertSidToStringSidW(Sid, SidText) then
      RaiseException(AccountSidUnformatted);
    try
      Result := CastIntegerToString(SidText);
    finally
      LocalFree(SidText);
    end;
  finally
    LocalFree(Sid);
  end;
end;

// Adds Sid to the managers recorded under RootKey\SubKey, once. The service
// grants a recorded user only while they stay in My Rig Managers.
procedure RecordManagerUser(RootKey: Integer; SubKey, Sid: String);
var
  Users: String;
begin
  if not RegQueryMultiStringValue(RootKey, SubKey, ManagerUsersValue, Users) then
    Users := '';
  if Pos(#0 + Sid + #0, #0 + Users + #0) > 0 then Exit;
  if Users <> '' then Users := Users + #0;
  if not RegWriteMultiStringValue(RootKey, SubKey, ManagerUsersValue, Users + Sid) then
    RaiseException(ManagerOwnershipUnrecorded);
end;

// Removes only a group this installer created; an already-deleted group is fine.
procedure RemoveManagerGroup;
begin
  RegDeleteValue(HKLM, OwnershipKey, ManagerUsersValue);
  if not RegValueExists(HKLM, OwnershipKey, ManagerGroupOwnedValue) then Exit;
  if RunSucceeded(SystemTool('net.exe'), 'localgroup ' + Quoted(ManagerGroup)) then
    RunRequired(SystemTool('net.exe'), 'localgroup ' + Quoted(ManagerGroup) + ' /delete');
  RegDeleteValue(HKLM, OwnershipKey, ManagerGroupOwnedValue);
end;

procedure EnsureManagerGroup;
var
  GroupName: String;
  Sid: DWORD_PTR;
  Status: DWORD;
begin
  GroupName := Quoted(ManagerGroup);
  if not RunSucceeded(SystemTool('net.exe'), 'localgroup ' + GroupName) then begin
    RunRequired(SystemTool('net.exe'), 'localgroup ' + GroupName + ' /add');
    if not RegWriteStringValue(HKLM, OwnershipKey, ManagerGroupOwnedValue, 'MyRig') then
      RaiseException(ManagerOwnershipUnrecorded);
  end else if not RegValueExists(HKLM, OwnershipKey, ManagerGroupOwnedValue) then
    RaiseException(ManagerGroupNotOwned);
  Sid := AccountSidPointer(InteractiveAccountName);
  try
    Status := NetLocalGroupAddMembers(0, ManagerGroup, 0, Sid, 1);
    if (Status <> 0) and (Status <> 1378) then
      RaiseException(ManagerNotAdded + IntToStr(Status));
  finally
    LocalFree(Sid);
  end;
  RecordManagerUser(HKLM, OwnershipKey, AccountSidText(InteractiveAccountName));
end;
