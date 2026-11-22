[Setup]
AppId=MyRig.AppAccess.Fixture
AppName=My Rig app access fixture
AppVersion=1
DefaultDirName={tmp}\MyRigAppAccessFixture
CreateAppDir=no
Uninstallable=no
PrivilegesRequired=lowest
OutputBaseFilename=app_access_fixture

[Code]
#include AddBackslash(CodeDir) + "install_messages.iss"
#include AddBackslash(CodeDir) + "install_utils.iss"
#include AddBackslash(CodeDir) + "app_files.iss"

function InitializeSetup: Boolean;
begin
  Result := False;
  GrantAppReadAccess(ExpandConstant('{param:OwnedDirectory}'));
  if not SaveStringToFile(ExpandConstant('{param:ResultFile}'), 'done', False) then
    RaiseException('Could not save the owned app access fixture result.');
end;
