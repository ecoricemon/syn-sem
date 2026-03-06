use super::util;
use crate::{error, Map, Result};
use std::{
    collections::hash_map,
    env, fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Default)]
pub struct AbstractFiles {
    files: Map<PathBuf, Source>,

    /// This set is a subset of `files`, which means all values in this map also belong to `files`.
    known: Map<Box<str>, Arc<str>>,
}

impl AbstractFiles {
    /// Returns true if the given path exists either in this container or physical file system.
    pub(crate) fn exists(&self, path: &Path) -> bool {
        self.files.contains_key(path) || path.exists()
    }

    pub(crate) fn insert_virtual_file(
        &mut self,
        virtual_path: PathBuf,
        code: Arc<str>,
    ) -> Option<Arc<str>> {
        self.files
            .insert(virtual_path, Source::Virtual(code))
            .map(|source| match source {
                Source::Physical(code) => code,
                Source::Virtual(code) => code,
            })
    }

    pub(crate) fn read(&mut self, path: &Path) -> Result<&Arc<str>> {
        // Doesn't exist? then finds the path then read it from the file system.
        if !self.files.contains_key(path) {
            let abs_path = Self::to_absolute_fs_path(path)?;
            let rel_path = Self::to_relative_fs_path(path)?.to_path_buf();
            let code: Arc<str> = fs::read_to_string(path)?.into();

            self.files.insert(abs_path, Source::Physical(code.clone()));
            self.files.insert(rel_path, Source::Physical(code));
        }

        match self.files.get(path).unwrap() {
            Source::Physical(code) => Ok(code),
            Source::Virtual(code) => Ok(code),
        }
    }

    pub(crate) fn to_absolute_path(&self, path: &Path) -> Result<PathBuf> {
        // Virtual file path cannot have its absolute path.
        if matches!(self.files.get(path), Some(Source::Virtual(_))) {
            Ok(path.to_path_buf())
        } else {
            Self::to_absolute_fs_path(path)
        }
    }

    pub(crate) fn to_relative_path<'a>(&self, path: &'a Path) -> Result<&'a Path> {
        // Virtual file path cannot have its relative path.
        if matches!(self.files.get(path), Some(Source::Virtual(_))) {
            Ok(path)
        } else {
            Self::to_relative_fs_path(path)
        }
    }

    pub(crate) fn to_name_path(&self, path: &Path) -> Result<String> {
        let rel_path = self.to_relative_path(path)?;

        let mut buf = String::with_capacity(rel_path.as_os_str().len());

        let crate_name = util::get_crate_name();
        buf.push_str(&crate_name);
        buf.push_str("::");
        for segment in rel_path.components() {
            if matches!(segment, std::path::Component::RootDir) {
                continue;
            }
            if !buf.ends_with("::") {
                buf.push_str("::");
            }
            let segment = util::os_str_to_str(segment.as_os_str())?;
            let segment = segment.trim_end_matches(".rs");
            buf.push_str(segment);
        }
        Ok(buf)
    }

    pub(crate) fn is_mod_rs(&self, path: &Path) -> bool {
        if !self.files.contains_key(path) && !path.is_file() {
            return false;
        }
        path.ends_with("mod.rs") || path.ends_with("lib.rs") || path.ends_with("main.rs")
    }

    pub(crate) fn known_libraries(&self) -> hash_map::Iter<'_, Box<str>, Arc<str>> {
        self.known.iter()
    }

    /// * name - e.g. "std"
    pub(crate) fn is_known_library(&self, name: &str) -> bool {
        #[cfg(debug_assertions)]
        if name.ends_with(".rs") {
            panic!(
                "expected {}, but received {name}",
                name.strip_suffix(".rs").unwrap()
            );
        }

        self.known.contains_key(name)
    }

    /// * name - e.g. "std"
    pub(crate) fn set_known_library(
        &mut self,
        name: Box<str>,
        path: &Path,
    ) -> Result<Option<Arc<str>>> {
        #[cfg(debug_assertions)]
        if name.ends_with(".rs") {
            panic!(
                "expected {}, but received {name}",
                name.strip_suffix(".rs").unwrap()
            );
        }

        let code = self.read(path)?.clone();
        let old_code = self.known.insert(name, code);
        Ok(old_code)
    }

    // Using 'env::args()' can be used to find more specific root directory than
    // 'env::current_dir()' because 'env::args()' gives us the exact entry file path. But it only
    // works on build process, not on macro expansion because there's no arguments during macro
    // expansion.
    fn to_absolute_fs_path(path: &Path) -> Result<PathBuf> {
        if path.is_absolute() {
            path.canonicalize().map_err(|e| {
                let path = path.to_string_lossy();
                error!("`{path}`: {e}")
            })
        } else {
            let path = env::current_dir()?.join(path);
            path.canonicalize().map_err(|e| {
                let path = path.to_string_lossy();
                match e.kind() {
                    io::ErrorKind::NotFound => error!("couldn't find `{path}`: {e}",),
                    _ => error!("`{path}`: {e}"),
                }
            })
        }
    }

    fn to_relative_fs_path(path: &Path) -> Result<&Path> {
        let rel_path = if path.is_relative() {
            path
        } else {
            let cur_dir = env::current_dir()?;
            path.strip_prefix(cur_dir)?
        };
        Ok(rel_path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    Physical(Arc<str>),
    Virtual(Arc<str>),
}
