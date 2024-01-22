use std::fs::{File, remove_file};
use std::io::{self, Write};
use sha1::{Sha1, Digest};
use std::process::Command;
use std::path::{PathBuf};
use std::env;
use crate::fileio::{note_path, temp_note_path, file_content};

pub struct Note {
    content: String,
    hash: String,
    parent_dir: Option<PathBuf>,
}

#[inline(always)]
pub fn open_temp_editor(content:String, path:&PathBuf) -> io::Result<String>{
    let mut file = File::create(path)?;
    write!(file, "{content}")?;
    let editor = if cfg!(windows) {
        String::from("notepad")
    } else {
         match env::var("EDITOR") {
            Ok(editor) => editor,
            Err(_) => String::from("vi")
        }
    };
    Command::new(editor).arg(path.clone()).status().expect("Couldn't open the editor.");
    let content = file_content(path)?;
    remove_file(path).unwrap();
    Ok(content)
}

impl Note {
    pub fn new(content:String, parent_dir: Option<PathBuf>)-> Self {
        let hash = sha1(&content);

        Note {
            content,
            hash,
            parent_dir,
        }
    }

    pub fn from_editor(parent_dir: Option<PathBuf>)-> io::Result<Self> {

        let content = open_temp_editor(String::new(), &temp_note_path())?;

        Ok(Note::new(content, parent_dir))
    }

    pub fn content(&self) -> String {
        self.content.clone()
    }

    pub fn from_hash(hash:&String, parent_dir: Option<PathBuf>) -> io::Result<Option<Self>> {
        if let Some(path) = note_path(&hash, parent_dir.clone())? {
            let content = file_content(&path)?;
            return Ok(Some(Note::new(content, parent_dir)))
        }
        Ok(None)
    }

    fn path(&self) -> Option<PathBuf> {
        note_path(&self.hash, self.parent_dir.clone()).expect("Unable to get the note's path.")
    }


    pub fn save(&self) -> io::Result<()> {
        if let Some(path) = self.path() {
            let mut file = File::create(path.as_os_str())?;
            file.write_all(self.content.as_bytes())?;
        }

        Ok(())
    }

    pub fn remove_file(&self) -> io::Result<()>{
        if let Some(path) = self.path() {
            remove_file(path)?;
        }
        Ok(())
    }

    pub fn edit_with_editor(&mut self) -> io::Result<()> {
        self.content = open_temp_editor(self.content.clone(), &temp_note_path())?;
        self.remove_file();
        self.hash = sha1(&self.content);
        Ok(())
    }

    pub fn hash(&self) -> String {
        self.hash.clone()
    }
}

pub fn sha1(str:&String) -> String{
    let mut hasher = Sha1::new();
    hasher.update(str);
    let result = hasher.finalize();
    format!("{:x}", result)
}
