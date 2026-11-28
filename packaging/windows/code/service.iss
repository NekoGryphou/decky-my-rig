function ServiceExists: Boolean;
begin
  Result := RegKeyExists(HKLM, ServiceKey);
end;

procedure VerifyOwnedService;
var
  ImagePath: String;
begin
  if not ServiceExists then Exit;
  if not RegQueryStringValue(HKLM, ServiceKey, 'ImagePath', ImagePath) or
    (CompareText(ImagePath, Quoted(ExpandConstant('{app}\myrig-service.exe'))) <> 0) then
    RaiseException(ServiceNotOwned);
end;

// - sc.exe prints the numeric state code in every Windows language.
function ServiceState: Integer;
var
  ExitCode, I, StateCode: Integer;
  Output: TExecOutput;
  Line: String;
begin
  if not ExecAndCaptureOutput(SystemTool('sc.exe'), 'query ' + ServiceName,
    '', SW_HIDE, ewWaitUntilTerminated, ExitCode, Output) or (ExitCode <> 0) or
    Output.Error then
    RaiseException(ServiceStateUnknown);
  for I := 0 to GetArrayLength(Output.StdOut) - 1 do begin
    Line := Uppercase(Trim(Output.StdOut[I]));
    for StateCode := 1 to 7 do
      if Pos(': ' + IntToStr(StateCode) + ' ', Line) > 0 then begin
        Result := StateCode;
        Exit;
      end;
  end;
  RaiseException(ServiceStateMissing);
end;

function IsServiceStopped: Boolean;
begin
  Result := ServiceState = 1;
end;

procedure WaitForServiceStopped;
var
  Deadline: DWORD;
begin
  Deadline := GetTickCount + ServiceStateTimeoutMs;
  while not IsServiceStopped do begin
    if GetTickCount >= Deadline then RaiseException(ServiceStopTimeout);
    Sleep(200);
  end;
end;

// ? A service already stopping refuses `sc stop`; waiting covers both cases.
procedure StopServiceAndWait;
begin
  if IsServiceStopped then Exit;
  RunSucceeded(SystemTool('sc.exe'), 'stop ' + ServiceName);
  WaitForServiceStopped;
end;

procedure WaitForServiceRunning;
var
  Deadline: DWORD;
  StateCode: Integer;
begin
  Deadline := GetTickCount + ServiceStateTimeoutMs;
  repeat
    StateCode := ServiceState;
    if StateCode = 4 then Exit;
    if StateCode = 1 then RaiseException(ServiceStoppedDuringStart);
    if GetTickCount >= Deadline then RaiseException(ServiceStartTimeout);
    Sleep(200);
  until False;
end;

procedure GrantServiceManagerRights;
var
  Sddl, Updated: String;
  ExitCode, I: Integer;
  Output: TExecOutput;
begin
  if not ExecAndCaptureOutput(SystemTool('sc.exe'), 'sdshow ' + ServiceName,
    '', SW_HIDE, ewWaitUntilTerminated, ExitCode, Output) or (ExitCode <> 0) or
    Output.Error then
    RaiseException(ServiceRightsUnreadable);
  Sddl := '';
  for I := 0 to GetArrayLength(Output.StdOut) - 1 do
    if Pos('D:', Trim(Output.StdOut[I])) = 1 then
      Sddl := Trim(Output.StdOut[I]);
  if Sddl = '' then RaiseException(ServiceRightsUnavailable);
  Updated := WithManagerAce(Sddl, AccountSidText(GetComputerNameString + '\' + ManagerGroup));
  if Updated <> Sddl then
    RunRequired(SystemTool('sc.exe'), 'sdset ' + ServiceName + ' ' + Updated);
end;

procedure EnsureService;
var
  Image, Parameters: String;
begin
  Image := ExpandConstant('{app}\myrig-service.exe');
  if ServiceExists then
    Parameters := 'config ' + ServiceName + ' binPath= ' + Quoted('\"' + Image + '\"') +
      ' start= auto obj= ' + Quoted('NT AUTHORITY\LocalService')
  else
    Parameters := 'create ' + ServiceName + ' binPath= ' + Quoted('\"' + Image + '\"') +
      ' start= auto obj= ' + Quoted('NT AUTHORITY\LocalService') +
      ' DisplayName= ' + Quoted('My Rig Host');
  RunRequired(SystemTool('sc.exe'), Parameters);
  RunRequired(SystemTool('sc.exe'), 'sidtype ' + ServiceName + ' unrestricted');
  RunRequired(SystemTool('sc.exe'), 'privs ' + ServiceName +
    ' SeChangeNotifyPrivilege/SeShutdownPrivilege');
  // ? A fatal error exits with an error code, not a crash. The failure flag makes Windows restart
  // ? the service either way. The one-day reset period caps the restarts, so a configuration that
  // ? cannot load does not loop.
  RunRequired(SystemTool('sc.exe'),
    'failure ' + ServiceName + ' reset= 86400 actions= restart/5000/restart/60000//');
  RunRequired(SystemTool('sc.exe'), 'failureflag ' + ServiceName + ' 1');
  GrantServiceManagerRights;
end;
