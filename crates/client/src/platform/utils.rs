use uuid::Uuid;

/// A fresh name for the file a save writes before renaming it over
/// `pcs.json`.
pub fn temporary_document_name() -> String {
    format!(".pcs-{}.tmp", Uuid::new_v4().simple())
}

/// Whether `name` is one of those files. One left behind by a killed save holds
/// every token, so the next save deletes it.
pub fn is_temporary_document(name: &str) -> bool {
    name.starts_with(".pcs-") && name.ends_with(".tmp")
}
