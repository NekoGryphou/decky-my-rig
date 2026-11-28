use std::{fs, path::Path};
use uuid::Uuid;

/// A fresh name for the file a save writes before renaming it over
/// `pcs.json`.
pub fn temporary_document_name() -> String {
    format!(".pcs-{}.tmp", Uuid::new_v4().simple())
}

/// Whether `name` is one of those files. One left behind by a killed save holds
/// every token, so the next save deletes it.
pub(crate) fn is_temporary_document(name: &str) -> bool {
    name.starts_with(".pcs-") && name.ends_with(".tmp")
}

/// Deletes the temp files in `directory` that killed saves left behind. A save
/// calls it under the store lock, so none belongs to a save in progress.
pub fn remove_leftover_documents(directory: &Path) {
    for entry in fs::read_dir(directory).into_iter().flatten().flatten() {
        if entry
            .file_name()
            .to_str()
            .is_some_and(is_temporary_document)
        {
            // * Best effort: a leftover that stays is retried at the next save.
            let _ = fs::remove_file(entry.path());
        }
    }
}
