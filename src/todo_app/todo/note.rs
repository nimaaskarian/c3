use std::fs::{File, remove_file};
use std::io::{self, Write};
use sha1::{Sha1, Digest};
use std::process::Command;
use std::env;
use crate::fileio::{file_content, temp_note_path};

#[inline(always)]
pub fn open_temp_editor(content:Option<&String>) -> io::Result<String>{
    let path = temp_note_path();
    let mut file = File::create(&path)?;
    if let Some(content) = content {
        write!(file, "{content}")?;
    }
    let editor = if cfg!(windows) {
        String::from("notepad")
    } else {
         match env::var("EDITOR") {
            Ok(editor) => editor,
            Err(_) => String::from("vi")
        }
    };
    Command::new(editor).arg(&path).status().expect("Couldn't open the editor.");
    let content = file_content(&path)?;
    remove_file(path).unwrap();
    Ok(content)
}

pub fn sha1(str:&String) -> String{
    let mut hasher = Sha1::new();
    hasher.update(str);
    let result = hasher.finalize();
    format!("{:x}", result)
}
