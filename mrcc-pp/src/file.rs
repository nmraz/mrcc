use std::borrow::Borrow;
use std::collections::hash_map::Entry;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use rustc_hash::FxHashMap;

use mrcc_source::smap::FileContents;

/// Represents the two kinds of `#include` directives.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum IncludeKind {
    /// `#include "filename"`
    Quoted,
    /// `#include <filename>`
    Angled,
}

/// Represents a source file loaded by the preprocessor.
pub struct File {
    /// The contents of the file.
    pub contents: Rc<FileContents>,
    /// The parent directory of the file, for use when resolving quoted `#include` directives.
    pub parent_dir: Option<PathBuf>,
}

impl File {
    /// Creates a new file with the specified data.
    pub fn new(contents: Rc<FileContents>, parent_dir: Option<PathBuf>) -> Rc<Self> {
        Rc::new(File {
            contents,
            parent_dir,
        })
    }
}

/// A path-based cache of loaded files.
struct FileCache {
    files: FxHashMap<PathBuf, Rc<File>>,
}

impl FileCache {
    /// Creates a new, empty cache.
    pub fn new() -> Self {
        Self {
            files: FxHashMap::default(),
        }
    }

    /// Loads the file at `path` into the cache and returns it.
    ///
    /// Subsequent loads of `path` will return the existing cached file.
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

/// Represents the errors that can occur when including a file.
pub enum IncludeError {
    /// The file was not found after searching all include paths.
    NotFound,
    /// An IO error occurred when reading the file.
    Io {
        full_path: PathBuf,
        error: io::Error,
    },
}

/// A structure responsible for finding and caching included files.
pub struct IncludeLoader {
    cache: FileCache,
    include_dirs: Vec<PathBuf>,
}

impl IncludeLoader {
    /// Creates a new include loader with the specified include directories.
    ///
    /// These will be searched in order when attempting to load an included file.
    pub fn new(include_dirs: Vec<PathBuf>) -> Self {
        Self {
            cache: FileCache::new(),
            include_dirs,
        }
    }

    /// Attempts to load the requested file, searching all include directories in order.
    ///
    /// If the include is a quoted include, the includer's parent directory is searched as well.
    pub fn load(
        &mut self,
        filename: &Path,
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

        if filename.is_absolute() {
            // Avoid repeatedly looking up the same file.
            return do_load(&mut self.cache, filename);
        }

        let initial_dir = includer
            .parent_dir
            .as_ref()
            .filter(|_| kind == IncludeKind::Quoted);

        let dirs = initial_dir.into_iter().chain(self.include_dirs.iter());

        for dir in dirs {
            match do_load(&mut self.cache, dir.join(filename)) {
                Err(IncludeError::NotFound) => continue,
                ret => return ret,
            }
        }

        Err(IncludeError::NotFound)
    }
}
