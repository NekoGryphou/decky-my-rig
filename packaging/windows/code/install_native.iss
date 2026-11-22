function LookupAccountNameW(SystemName, AccountName: String; Sid: DWORD_PTR;
  var SidSize: DWORD; DomainName: String; var DomainSize: DWORD;
  var SidUse: DWORD): Boolean;
  external 'LookupAccountNameW@advapi32.dll stdcall';

function ConvertSidToStringSidW(Sid: DWORD_PTR; var SidText: DWORD_PTR): Boolean;
  external 'ConvertSidToStringSidW@advapi32.dll stdcall';

function LocalAlloc(Flags: DWORD; Bytes: DWORD): DWORD_PTR;
  external 'LocalAlloc@kernel32.dll stdcall';

function LocalFree(Memory: DWORD_PTR): DWORD_PTR;
  external 'LocalFree@kernel32.dll stdcall';

function NetLocalGroupAddMembers(ServerName: DWORD_PTR; GroupName: String;
  Level: DWORD; var MemberSid: DWORD_PTR; Count: DWORD): DWORD;
  external 'NetLocalGroupAddMembers@netapi32.dll stdcall';

function GetCurrentProcessId: DWORD;
  external 'GetCurrentProcessId@kernel32.dll stdcall';

function ProcessIdToSessionId(ProcessId: DWORD; var SessionId: DWORD): Boolean;
  external 'ProcessIdToSessionId@kernel32.dll stdcall';

function WTSQuerySessionInformationW(Server: DWORD_PTR; SessionId, InfoClass: DWORD;
  var Buffer: DWORD_PTR; var Bytes: DWORD): Boolean;
  external 'WTSQuerySessionInformationW@wtsapi32.dll stdcall';

procedure WTSFreeMemory(Buffer: DWORD_PTR);
  external 'WTSFreeMemory@wtsapi32.dll stdcall';

function GetTickCount: DWORD;
  external 'GetTickCount@kernel32.dll stdcall';
