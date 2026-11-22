use std::path::PathBuf;
use windows::Win32::{
    System::Com::CoTaskMemFree,
    UI::Shell::{FOLDERID_ProgramData, KNOWN_FOLDER_FLAG, SHGetKnownFolderPath},
};

pub(crate) fn program_data() -> Result<PathBuf, &'static str> {
    const FAILURE: &str = "ProgramData is unavailable.";
    let pointer =
        // ! SAFETY: FOLDERID_ProgramData is a static GUID; the returned string is freed below.
        unsafe { SHGetKnownFolderPath(&FOLDERID_ProgramData, KNOWN_FOLDER_FLAG(0), None) }
            .map_err(|_| FAILURE)?;
    // ! SAFETY: `pointer` is the NUL-terminated string SHGetKnownFolderPath returned.
    let result = unsafe { pointer.to_string() }
        .map(PathBuf::from)
        .map_err(|_| FAILURE);
    // ! SAFETY: `pointer` came from SHGetKnownFolderPath and is freed exactly once.
    unsafe {
        CoTaskMemFree(Some(pointer.0.cast()));
    }
    result
}
