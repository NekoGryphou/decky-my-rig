function Quoted(Value: String): String;
begin
  Result := '"' + Value + '"';
end;

function SystemTool(Name: String): String;
begin
  Result := ExpandConstant('{sys}\') + Name;
end;

function ConfigDirectory: String;
begin
  Result := ExpandConstant('{commonappdata}\MyRig');
end;

function SecurityDirectory: String;
begin
  Result := ExpandConstant('{commonappdata}\MyRigSecurity');
end;

procedure RunRequired(Executable, Arguments: String);
var
  ExitCode: Integer;
begin
  if not Exec(Executable, Arguments, '', SW_HIDE, ewWaitUntilTerminated, ExitCode) or
     (ExitCode <> 0) then
    RaiseException(SetupStepFailed + ExtractFileName(Executable) + ' (' + IntToStr(ExitCode) + ').');
end;

function RunSucceeded(Executable, Arguments: String): Boolean;
var
  ExitCode: Integer;
begin
  Result := Exec(Executable, Arguments, '', SW_HIDE, ewWaitUntilTerminated, ExitCode) and
    (ExitCode = 0);
end;
