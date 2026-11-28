[Setup]
AppId=MyRig.ServiceAcl.Fixture
AppName=My Rig service ACL fixture
AppVersion=1
DefaultDirName={tmp}\MyRigServiceAclFixture
CreateAppDir=no
Uninstallable=no
PrivilegesRequired=lowest
OutputBaseFilename=service_acl_fixture

[Code]
#include AddBackslash(CodeDir) + "install_native.iss"
#include AddBackslash(CodeDir) + "service_rights.iss"

const
  ManagerSid = 'S-1-5-21-100-200-300-1001';

function ConvertStringSecurityDescriptorToSecurityDescriptorW(Sddl: String;
  Revision: DWORD; var Descriptor: DWORD_PTR; var Bytes: DWORD): Boolean;
  external 'ConvertStringSecurityDescriptorToSecurityDescriptorW@advapi32.dll stdcall';

function ConvertSecurityDescriptorToStringSecurityDescriptorW(Descriptor: DWORD_PTR;
  Revision, SecurityInformation: DWORD; var Text: DWORD_PTR; var Characters: DWORD): Boolean;
  external 'ConvertSecurityDescriptorToStringSecurityDescriptorW@advapi32.dll stdcall';

// - What `sc sdshow` prints after Windows stores the descriptor.
function CanonicalSddl(Sddl: String): String;
var
  Descriptor, Text: DWORD_PTR;
  Bytes, Characters: DWORD;
begin
  Descriptor := 0;
  Text := 0;
  if not ConvertStringSecurityDescriptorToSecurityDescriptorW(Sddl, 1, Descriptor, Bytes) then
    RaiseException('The fixture descriptor is invalid.');
  try
    if not ConvertSecurityDescriptorToStringSecurityDescriptorW(Descriptor, 1, 4, Text, Characters) then
      RaiseException('Windows could not serialize the fixture descriptor.');
    try
      Result := CastIntegerToString(Text);
    finally
      LocalFree(Text);
    end;
  finally
    LocalFree(Descriptor);
  end;
end;

// - Grants twice through Windows' normalization and counts the writes.
function InitializeSetup: Boolean;
var
  Stored, Updated: String;
  Writes, Round: Integer;
begin
  Result := False;
  Writes := 0;
  Stored := CanonicalSddl('D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)');
  for Round := 1 to 2 do begin
    Updated := WithManagerAce(Stored, ManagerSid);
    if Updated <> Stored then begin
      Writes := Writes + 1;
      Stored := CanonicalSddl(Updated);
    end;
  end;
  if not SaveStringToFile(ExpandConstant('{param:ResultFile}'), IntToStr(Writes), False) then
    RaiseException('Could not save the owned service ACL fixture result.');
end;
