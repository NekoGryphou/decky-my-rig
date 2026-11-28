/// SYSTEM and Administrators, not inherited, so no other account can open the
/// folder between creating it and the installer's grants. The owner gets read
/// control only, as the installer's OWNER RIGHTS grant does; the data folder
/// check refuses write access for any principal that is not trusted. When the
/// installer runs elevated, the owner is Administrators.
pub(super) const ADMINISTRATORS_ONLY: &str = "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;;RC;;;OW)";
