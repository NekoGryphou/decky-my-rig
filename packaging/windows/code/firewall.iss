// Returns the owned rule in Rule, or False when there is none. A rule with
// that name for another program is never touched.
function FindOwnedFirewallRule(Policy: Variant; var Rule: Variant): Boolean;
begin
  Result := False;
  try
    Rule := Policy.Rules.Item(FirewallRuleName);
    Result := True;
  except
    Log('No MyRig firewall rule.');
  end;
  if Result and (CompareText(Rule.ApplicationName,
    ExpandConstant('{app}\myrig-service.exe')) <> 0) then
    RaiseException(FirewallRuleNotOwned);
end;

procedure ConfigureFirewallRule(Rule: Variant);
begin
  Rule.Description := 'My Rig host';
  Rule.ApplicationName := ExpandConstant('{app}\myrig-service.exe');
  Rule.Protocol := 6;
  Rule.LocalPorts := '*';
  Rule.Profiles := FirewallAllProfiles;
  Rule.RemoteAddresses := 'LocalSubnet';
  Rule.Direction := 1;
  Rule.Action := 1;
  Rule.Enabled := True;
end;

// Updates an existing owned rule in place, so a failure never leaves the
// service without one.
procedure EnsureFirewall;
var
  Policy, Rule: Variant;
begin
  Policy := CreateOleObject('HNetCfg.FwPolicy2');
  if FindOwnedFirewallRule(Policy, Rule) then
    ConfigureFirewallRule(Rule)
  else begin
    Rule := CreateOleObject('HNetCfg.FWRule');
    Rule.Name := FirewallRuleName;
    ConfigureFirewallRule(Rule);
    Policy.Rules.Add(Rule);
    FirewallCreated := True;
  end;
end;

procedure RemoveOwnedFirewallRule;
var
  Policy, Rule: Variant;
begin
  Policy := CreateOleObject('HNetCfg.FwPolicy2');
  if FindOwnedFirewallRule(Policy, Rule) then
    Policy.Rules.Remove(FirewallRuleName);
end;
