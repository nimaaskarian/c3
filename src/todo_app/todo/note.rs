use std::path::PathBuf;
use std::io;
use sha1::{Sha1, Digest};
use crate::fileio::{temp_path, open_temp_editor};

pub fn open_note_temp_editor(content:Option<&str>) -> io::Result<String>{
    open_temp_editor(content,temp_note_path())
}

#[inline(always)]
pub fn temp_note_path() -> PathBuf{
    temp_path("note")
}

pub fn sha1(str:&str) -> String{
    let mut hasher = Sha1::new();
    hasher.update(str);
    let result = hasher.finalize();
    format!("{:x}", result)
}
