#define AppGuid "00000000-0000-4000-8000-000000000000"
[Setup]
AppId=MyRig.Firewall.Fixture
AppName=My Rig firewall fixture
AppVersion=1
DefaultDirName={tmp}\MyRigFirewallFixture
CreateAppDir=no
Uninstallable=no
PrivilegesRequired=lowest
OutputBaseFilename=firewall_fixture

[Code]
var
  FirewallCreated: Boolean;

#include AddBackslash(CodeDir) + "install_messages.iss"
#include AddBackslash(CodeDir) + "install_consts.iss"
#include AddBackslash(CodeDir) + "install_utils.iss"
#include AddBackslash(CodeDir) + "firewall.iss"

// - Applies the installer's real rule settings to a scratch rule, reads them back
//   through the firewall API, and removes the rule. The real MyRig rule is never
//   touched; the scratch name differs from FirewallRuleName.
function InitializeSetup: Boolean;
var
  Policy, Rule: Variant;
  Scope, Ports, App: String;
  Profiles, Action, Direction, Protocol: Integer;
  Enabled: Boolean;
  Summary, EnabledText: String;
begin
  Result := False;
  Policy := CreateOleObject('HNetCfg.FwPolicy2');
  Rule := CreateOleObject('HNetCfg.FWRule');
  Rule.Name := 'MyRig-test-firewall';
  // ? Same order as the installer's create path: configure, then add.
  ConfigureFirewallRule(Rule, 'C:\MyRig\myrig-service.exe');
  Policy.Rules.Add(Rule);
  Rule := Policy.Rules.Item('MyRig-test-firewall');
  Profiles := Rule.Profiles;
  Scope := Rule.RemoteAddresses;
  Action := Rule.Action;
  Direction := Rule.Direction;
  Protocol := Rule.Protocol;
  Ports := Rule.LocalPorts;
  Enabled := Rule.Enabled;
  if Enabled then EnabledText := 'yes' else EnabledText := 'no';
  App := Rule.ApplicationName;
  Summary := 'profiles=' + IntToStr(Profiles) + ';scope=' + Scope +
    ';action=' + IntToStr(Action) + ';direction=' + IntToStr(Direction) +
    ';protocol=' + IntToStr(Protocol) + ';ports=' + Ports +
    ';enabled=' + EnabledText + ';app=' + App;
  Policy.Rules.Remove('MyRig-test-firewall');
  if not SaveStringToFile(ExpandConstant('{param:ResultFile}'), Summary, False) then
    RaiseException('Could not save the firewall fixture result.');
end;
