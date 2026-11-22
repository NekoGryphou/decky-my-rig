/// SYSTEM, Administrators and the folder's owner, not inherited, so no other
/// account can open the folder between creating it and the installer's grants.
/// When the installer runs elevated, the owner is Administrators.
pub(super) const ADMINISTRATORS_ONLY: &str = "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;;FA;;;OW)";
