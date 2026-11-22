[Setup]
AppId=MyRig.Directory.Fixture
AppName=My Rig directory admission fixture
AppVersion=1
DefaultDirName={tmp}\MyRigDirectoryFixture
CreateAppDir=no
Uninstallable=no
PrivilegesRequired=lowest
OutputBaseFilename=directory_fixture

[Files]
Source: "{#HelperExe}"; Flags: dontcopy

[Code]
#include AddBackslash(CodeDir) + "install_messages.iss"
#include AddBackslash(CodeDir) + "install_utils.iss"

function RegisteredAppDirectory: String;
begin
  Result := ExpandConstant('{param:ExistingApp|}');
end;

#include AddBackslash(CodeDir) + "directory.iss"

function InitializeSetup: Boolean;
var
  Failure: String;
begin
  Result := False;
  Failure := CheckInstallationDirectory(ExpandConstant('{param:TestPath}'));
  if not SaveStringToFile(ExpandConstant('{param:ResultFile}'), Failure, False) then
    RaiseException('Could not save the owned directory fixture result.');
end;
