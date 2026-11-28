// ? After a failed reinstall the previous configuration is still in place, so start the service.
procedure RestartStoppedService;
begin
  try
    if ServiceWasStopped and ServiceExists then
      RunRequired(SystemTool('sc.exe'), 'start ' + ServiceName);
  except
    Log('Could not restart the service after the failed reinstall: ' + GetExceptionMessage);
  end;
end;

// - Best effort: removes what a failed fresh install created, logging what it can't.
procedure RollbackFreshInstall;
begin
  try
    if ServiceExists then begin
      VerifyOwnedService;
      StopServiceAndWait;
      RunRequired(SystemTool('sc.exe'), 'delete ' + ServiceName);
    end;
  except
    Log('Could not roll back the new service: ' + GetExceptionMessage);
  end;
  try
    if FirewallCreated then RemoveOwnedFirewallRule;
  except
    Log('Could not roll back the new firewall rule: ' + GetExceptionMessage);
  end;
  try
    if not GroupWasPresent then RemoveManagerGroup;
  except
    Log('Could not roll back the new manager group: ' + GetExceptionMessage);
  end;
  // ? Only folders this run created: one another account made is not ours.
  if not DataCreated then Exit;
  if not DelTree(ConfigDirectory, True, True, True) then
    Log('Could not roll back the new configuration directory.');
  if not DelTree(SecurityDirectory, True, True, True) then
    Log('Could not roll back the new security directory.');
end;
