pub mod doc;
pub mod editor;
pub mod window;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub markdown: String,
    pub text: String,
    pub created_at: f64,
    pub updated_at: f64,
    pub last_accessed_at: f64,
    pub pinned: bool,
}

pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("notes.json")
}

pub fn load(data_dir: &Path) -> Vec<Note> {
    crate::store::read_json(&path(data_dir))
}

pub fn save(data_dir: &Path, notes: &[Note]) -> std::io::Result<()> {
    crate::store::write_json(&path(data_dir), &notes)
}

pub fn current(notes: &[Note]) -> Option<usize> {
    notes.iter().enumerate().max_by(|a, b| a.1.last_accessed_at.total_cmp(&b.1.last_accessed_at)).map(|(i, _)| i)
}

pub fn uuid() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("system random source");
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

pub fn new_note(now: f64) -> Note {
    Note { id: uuid(), created_at: now, updated_at: now, last_accessed_at: now, ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_json_round_trips_in_the_web_format() {
        let dir = tempfile::tempdir().unwrap();
        let json = r##"[{"id":"a","markdown":"# Hi","text":"Hi","createdAt":1,"updatedAt":2,"lastAccessedAt":5,"pinned":true},{"id":"b","markdown":"","text":"","createdAt":1,"updatedAt":1,"lastAccessedAt":3,"pinned":false}]"##;
        std::fs::write(path(dir.path()), json).unwrap();
        let notes = load(dir.path());
        assert_eq!(current(&notes), Some(0));
        save(dir.path(), &notes).unwrap();
        assert_eq!(load(dir.path()), notes);
        let id = uuid();
        assert_eq!((id.len(), &id[14..15]), (36, "4"));
    }
}
