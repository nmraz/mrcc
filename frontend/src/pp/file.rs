use std::collections::hash_map::Entry;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::smap::FileContents;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum IncludeKind {
    Str,
    Angle,
}

pub struct File {
    pub contents: Rc<FileContents>,
    pub parent_dir: Option<PathBuf>,
}

impl File {
    pub fn new(contents: Rc<FileContents>, parent_dir: Option<PathBuf>) -> Rc<Self> {
        Rc::new(File {
            contents,
            parent_dir,
        })
    }
}

struct FileCache {
    files: FxHashMap<PathBuf, Rc<File>>,
}

impl FileCache {
    pub fn new() -> Self {
        Self {
            files: FxHashMap::default(),
        }
    }

    pub fn load(&mut self, path: impl AsRef<Path>) -> io::Result<Rc<File>> {
        let path = weakly_normalize(path.as_ref());

        match self.files.entry(path) {
            Entry::Occupied(ent) => Ok(ent.get().clone()),
            Entry::Vacant(ent) => {
                let path = ent.key();
                let contents = FileContents::new(&fs::read_to_string(&path)?);
                let file = File::new(contents, path.parent().map(|p| p.into()));
                ent.insert(file.clone());
                Ok(file)
            }
        }
    }
}

fn weakly_normalize(path: &Path) -> PathBuf {
    path.components()
        .filter(|&c| c != Component::CurDir)
        .collect()
}
