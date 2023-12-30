use std::fs::File;
use std::io::{prelude::*, self};
use std::path::PathBuf;
use home::home_dir;

#[inline(always)]
pub fn append_home_dir(str:&str) -> PathBuf {
    PathBuf::from(format!("{}/{}", home_dir().unwrap().to_str().unwrap(), str))
}

#[inline(always)]
pub fn note_path(hash:&String) -> PathBuf {
    append_home_dir(".local/share/calcurse/notes").join(hash)
}

#[inline(always)]
pub fn temp_note_path() -> PathBuf{
    let filename = format!("potato-note.{}", rand::random::<u16>());
    let path = home_dir().unwrap().join(filename);
    path.to_path_buf()
}

#[inline(always)]
pub fn file_content(path:&PathBuf) -> io::Result<String> {
    let mut content = String::new();
    let mut file = File::open(path)?;
    file.read_to_string(&mut content)?;
    Ok(content)
}

