#define AppGuid "00000000-0000-4000-8000-000000000000"
[Setup]
AppId=MyRig.ManagerUsers.Fixture
AppName=My Rig recorded managers fixture
AppVersion=1
DefaultDirName={tmp}\MyRigManagerUsersFixture
CreateAppDir=no
Uninstallable=no
PrivilegesRequired=lowest
OutputBaseFilename=manager_users_fixture

[Code]
#include AddBackslash(CodeDir) + "install_consts.iss"
#include AddBackslash(CodeDir) + "install_messages.iss"
#include AddBackslash(CodeDir) + "install_native.iss"
#include AddBackslash(CodeDir) + "install_utils.iss"
#include AddBackslash(CodeDir) + "managers.iss"

// - Records two users, one of them twice, under an owned HKCU key and saves
//   what was stored, one SID per line; the key is removed afterwards.
function InitializeSetup: Boolean;
var
  Key, Users: String;
begin
  Result := False;
  Key := 'Software\' + ExpandConstant('{param:FixtureKey}');
  try
    RecordManagerUser(HKCU, Key, 'S-1-5-21-100-200-300-1001');
    RecordManagerUser(HKCU, Key, 'S-1-5-21-100-200-300-1002');
    RecordManagerUser(HKCU, Key, 'S-1-5-21-100-200-300-1001');
    if not RegQueryMultiStringValue(HKCU, Key, ManagerUsersValue, Users) then Users := '';
    StringChangeEx(Users, #0, #13#10, True);
    if not SaveStringToFile(ExpandConstant('{param:ResultFile}'), Users, False) then
      RaiseException('Could not save the owned recorded managers fixture result.');
  finally
    RegDeleteKeyIncludingSubkeys(HKCU, Key);
  end;
end;
