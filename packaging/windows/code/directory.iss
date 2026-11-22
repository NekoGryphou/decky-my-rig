// Needs RegisteredAppDirectory, defined by the installer (storage.iss) or a fixture.
function DirectoryIsEmpty(Directory: String): Boolean;
var
  Entry: TFindRec;
begin
  Result := True;
  if FindFirst(AddBackslash(Directory) + '*', Entry) then begin
    try
      repeat
        if (Entry.Name <> '.') and (Entry.Name <> '..') then begin
          Result := False;
          Exit;
        end;
      until not FindNext(Entry);
    finally
      FindClose(Entry);
    end;
  end;
end;

function CheckInstallationDirectory(Directory: String): String;
var
  Previous: String;
  ExitCode: Integer;
  Output: TExecOutput;
begin
  Result := DirectorySecurityFailure;
  Directory := RemoveBackslashUnlessRoot(ExpandFileName(Directory));
  Previous := RegisteredAppDirectory;
  if (Previous <> '') and (CompareText(Directory, Previous) <> 0) then begin
    Result := DirectoryRelocation;
    Exit;
  end;
  if (Previous = '') and DirExists(Directory) and not DirectoryIsEmpty(Directory) then begin
    Result := DirectoryNotEmpty;
    Exit;
  end;
  try
    ExtractTemporaryFile('myrig-setup.exe');
    if ExecAndCaptureOutput(ExpandConstant('{tmp}\myrig-setup.exe'),
      'check-install-dir ' + Quoted(Directory), '', SW_HIDE, ewWaitUntilTerminated,
      ExitCode, Output) and (ExitCode = 0) and not Output.Error then Result := '';
  except
    Log('Installation directory security could not be verified.');
  end;
end;
