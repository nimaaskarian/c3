use crate::fileio::{open_temp_editor, temp_path};
use sha1::{Digest, Sha1};
use std::io;
use std::path::PathBuf;

pub fn open_note_temp_editor(content: Option<&str>) -> io::Result<String> {
    open_temp_editor(content, temp_note_path())
}

#[inline(always)]
pub fn temp_note_path() -> PathBuf {
    temp_path("note")
}

pub fn sha1(str: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(str);
    let result = hasher.finalize();
    format!("{:x}", result)
}
