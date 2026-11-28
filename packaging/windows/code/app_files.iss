// - LocalService and Users read and run the app files; nobody else gains write access.
procedure GrantAppReadAccess(Directory: String);
begin
  RunRequired(SystemTool('icacls.exe'), Quoted(Directory) +
    ' /grant:r *S-1-5-19:(OI)(CI)RX *S-1-5-32-545:(OI)(CI)RX /T');
  // ? Inheritable grants leave protected file ACLs unchanged, even with /T.
  RunRequired(SystemTool('icacls.exe'), Quoted(AddBackslash(Directory) + '*') +
    ' /grant *S-1-5-19:RX *S-1-5-32-545:RX /T');
end;
