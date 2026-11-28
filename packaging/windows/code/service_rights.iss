// - Returns Sddl with the managers' allow ACE, unchanged when Windows already
//   lists it; the ACE goes before any SACL part.
function WithManagerAce(Sddl, GroupSid: String): String;
var
  Ace: String;
  SaclStart: Integer;
begin
  Result := Sddl;
  Ace := '(A;;CCLCRPWPRC;;;' + GroupSid + ')';
  if Pos(Ace, Result) > 0 then Exit;
  SaclStart := Pos('S:', Result);
  if SaclStart > 0 then
    Insert(Ace, Result, SaclStart)
  else
    Result := Result + Ace;
end;
