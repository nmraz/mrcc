use std::borrow::Borrow;
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

    pub fn load(&mut self, path: &Path) -> io::Result<Rc<File>> {
        let path = weakly_normalize(path);
        match self.files.entry(path) {
            Entry::Occupied(ent) => Ok(ent.get().clone()),
            Entry::Vacant(ent) => {
                let path = ent.key();
                let file = File::new(
                    FileContents::new(&fs::read_to_string(&path)?),
                    path.parent().map(|p| p.into()),
                );
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

pub enum IncludeError {
    NotFound,
    Io {
        full_path: PathBuf,
        error: io::Error,
    },
}

pub struct IncludeLoader {
    cache: FileCache,
    include_paths: Vec<PathBuf>,
}

impl IncludeLoader {
    pub fn new(include_paths: Vec<PathBuf>) -> Self {
        Self {
            cache: FileCache::new(),
            include_paths,
        }
    }

    pub fn load(
        &mut self,
        path: &Path,
        kind: IncludeKind,
        includer: &File,
    ) -> Result<Rc<File>, IncludeError> {
        fn do_load(
            cache: &mut FileCache,
            full_path: impl Borrow<Path> + Into<PathBuf>,
        ) -> Result<Rc<File>, IncludeError> {
            cache.load(full_path.borrow()).map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    IncludeError::NotFound
                } else {
                    IncludeError::Io {
                        full_path: full_path.into(),
                        error: e,
                    }
                }
            })
        };

        if path.is_absolute() {
            return do_load(&mut self.cache, path);
        }

        let initial_dir = if kind == IncludeKind::Str {
            includer.parent_dir.as_ref()
        } else {
            None
        };

        let dirs = initial_dir.into_iter().chain(self.include_paths.iter());

        for dir in dirs {
            match do_load(&mut self.cache, dir.join(path)) {
                Err(IncludeError::NotFound) => continue,
                ret => return ret,
            }
        }

        Err(IncludeError::NotFound)
    }
}
