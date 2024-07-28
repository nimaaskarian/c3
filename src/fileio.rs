use home::home_dir;
use std::fs::{remove_dir, remove_file, File};
use std::io::{self, prelude::*, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use std::env;
use std::process::Command;

#[inline(always)]
pub fn append_notes_to_path_parent(filename: &Path) -> PathBuf {
    filename.parent().unwrap().join("notes")
}

#[inline(always)]
pub fn open_temp_editor(content: Option<&str>, path: PathBuf) -> io::Result<String> {
    let mut file = File::create(&path)?;
    if let Some(content) = content {
        write!(file, "{content}")?;
    }
    let editor = if cfg!(windows) {
        String::from("notepad")
    } else {
        env::var("EDITOR").unwrap_or(String::from("vim"))
    };
    Command::new(editor)
        .arg(&path)
        .status()
        .expect("Couldn't open the editor.");
    let content = file_content(&path)?;
    remove_file(path).unwrap();
    Ok(content)
}

#[inline(always)]
pub fn append_home_dir(vec: [&str; 4]) -> PathBuf {
    let mut path = home_dir().unwrap().to_path_buf();
    for item in vec {
        path = path.join(item);
    }

    path
}

#[inline(always)]
pub fn get_todo_path() -> io::Result<PathBuf> {
    let file = append_home_dir([".local", "share", "calcurse", "todo"]);
    if file.is_dir() {
        remove_dir(&file)?;
    }
    let parentdir = file.parent().unwrap();
    std::fs::create_dir_all(parentdir)?;
    Ok(file)
}

#[inline(always)]
pub fn temp_path(name: &str) -> PathBuf {
    let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Err(_) => 12345,
        Ok(some) => some.as_secs(),
    };
    let filename = format!("c3-{name}.{time}");
    let tmpdir = PathBuf::from("/tmp");
    let path = if tmpdir.is_dir() {
        tmpdir.join(filename)
    } else {
        home_dir().unwrap().join(filename)
    };
    path.to_path_buf()
}

#[inline(always)]
pub fn file_content(path: &Path) -> io::Result<String> {
    let mut content = String::new();
    let mut file = File::open(path)?;
    file.read_to_string(&mut content)?;
    Ok(content)
}
