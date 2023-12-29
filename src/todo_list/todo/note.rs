use sha1::{Sha1, Digest};
use std::fs::{File, remove_file};
use std::process::Command;
use std::path::{Path, PathBuf};
use std::io::{prelude::*, self};
use std::env;

pub struct Note {
    content: String,
    hash: String,
}

#[inline(always)]
pub fn hash_path(hash:&String) -> PathBuf {
    let home = env::var("HOME").unwrap();
    Path::new(&home).join(".local/share/calcurse/notes").join(hash)
}
pub enum NoteError {
    FileNotExists,
    UnableToRead,
}

#[inline(always)]
pub fn file_content(path:&PathBuf) -> Result<String, NoteError> {
    let mut content = String::new();
    let mut file = match File::open(path){
        Err(_) => return Err(NoteError::FileNotExists),
        Ok(file) => file,
    };
    if file.read_to_string(&mut content).is_ok() {
        Ok(content)
    } else {
        Err(NoteError::UnableToRead)
    }
}

impl Note {
    pub fn from_editor()-> Result<Self, NoteError> {
        let editor = match env::var("EDITOR") {
            Ok(editor) => editor,
            Err(_) => String::from("vi")
        };
        let home = env::var("HOME").unwrap();
        let filename = format!("potato-note.{}", rand::random::<u16>());

        let path = Path::new(&home).join(filename);

        Command::new(editor).arg(path.clone()).status().expect("Couldn't open the editor.");
        let content = file_content(&path)?;
        remove_file(path).unwrap();

        Ok(Note::new(content))
    }

    pub fn from_hash(hash:String) -> Result<Self, NoteError> {
        let content = file_content(&hash_path(&hash))?;
        Ok(Note::new(content))
    }

    fn path(&self) -> PathBuf {
        hash_path(&self.hash)
    }

    pub fn new(content:String)-> Self {
        let hash = sha1(&content);

        Note {
            content,
            hash,
        }
    }
    pub fn save(&self) -> io::Result<()> {
        let mut file = File::create(self.path())?;
        file.write_all(self.content.as_bytes())?;

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
