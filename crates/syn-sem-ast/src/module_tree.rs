use crate::{File, Item, ItemMod, SyntaxCx};
use std::path::{Path, PathBuf};
use syn_sem_common::{MaybeResult, Result, Set, Str};

/// Parsed source file used as input to upper semantic phases.
#[derive(Clone, Copy)]
pub struct SourceInput<'cx> {
    /// Interned source path for this parsed file.
    pub file_path: Str<'cx>,
    /// Parsed semantic AST for this file.
    pub file: &'cx File<'cx>,
}

pub(crate) struct ModuleTreeBuilder<'cx> {
    scx: &'cx SyntaxCx<'cx>,
    seen: Set<PathBuf>,
    files: Vec<SourceInput<'cx>>,
}

impl<'cx> ModuleTreeBuilder<'cx> {
    pub(crate) fn new(scx: &'cx SyntaxCx<'cx>) -> Self {
        Self {
            scx,
            seen: Set::default(),
            files: Vec::new(),
        }
    }

    pub(crate) fn collect(mut self, entry_path: Str<'cx>) -> Result<Vec<SourceInput<'cx>>> {
        let file = self.scx.lookup_source(entry_path)?.ast();
        let path = ModulePath::from_entry_file(PathBuf::from(entry_path.as_ref()));
        self.add_file(entry_path, file);
        self.scan_file(file, &path)?;
        Ok(self.files)
    }

    fn add_file(&mut self, file_path: Str<'cx>, file: &'cx File<'cx>) -> bool {
        if !self.seen.insert(PathBuf::from(file_path.as_ref())) {
            return false;
        }
        self.files.push(SourceInput { file_path, file });
        true
    }

    fn scan_file(&mut self, file: &'cx File<'cx>, path: &ModulePath) -> Result<()> {
        for item in file.items {
            self.scan_item(item, path)?;
        }
        Ok(())
    }

    fn scan_item(&mut self, item: &'cx Item<'cx>, path: &ModulePath) -> Result<()> {
        let Item::Mod(module) = item else {
            return Ok(());
        };

        if let Some(items) = module.items {
            let path = path.enter_inline_module(module)?;
            for item in items {
                self.scan_item(item, &path)?;
            }
            return Ok(());
        }

        let candidates = path.child_file_candidates(module)?;
        let Some(file_path) = self.find_child_file(&candidates)? else {
            return Ok(());
        };
        let file = self.scx.lookup_source(file_path)?.ast();
        if self.add_file(file_path, file) {
            let path = path.enter_external_module(module, PathBuf::from(file_path.as_ref()))?;
            self.scan_file(file, &path)?;
        }
        Ok(())
    }

    fn find_child_file(&self, candidates: &[PathBuf]) -> MaybeResult<Str<'cx>> {
        for path in candidates {
            let file_path = self.scx.common.intern_path(path);
            if self.scx.has_source(file_path) {
                return Ok(Some(file_path));
            }
        }

        for path in candidates {
            if path.is_file() {
                return self.scx.read_physical_file(path).map(Some);
            }
        }

        Ok(None)
    }
}

/// Tracks Rust module source locations while walking a module tree.
///
/// It records both the file currently being visited and the directory used to search for that
/// module's out-of-line child files, such as `foo.rs` or `foo/mod.rs`.
#[derive(Clone, Debug)]
pub struct ModulePath {
    source_file: PathBuf,
    module_dir: PathBuf,
}

impl ModulePath {
    /// Creates module path state for a crate entry file.
    pub fn from_entry_file(file_path: PathBuf) -> Self {
        let source_dir = file_path.parent().unwrap_or_else(|| Path::new(""));
        let stem = file_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("root module path must be a Rust source file path");
        let module_dir = match stem {
            "lib" | "main" | "mod" => source_dir.to_path_buf(),
            stem => source_dir.join(stem),
        };

        Self {
            source_file: file_path,
            module_dir,
        }
    }

    fn source_dir(&self) -> &Path {
        self.source_file.parent().unwrap_or_else(|| Path::new(""))
    }

    /// Returns path state for an inline child module.
    pub fn enter_inline_module(&self, module: &ItemMod<'_>) -> Result<Self> {
        Ok(Self {
            source_file: self.source_file.clone(),
            module_dir: self.child_dir(module)?,
        })
    }

    /// Returns path state for an out-of-line child module stored in `file_path`.
    pub fn enter_external_module(&self, module: &ItemMod<'_>, file_path: PathBuf) -> Result<Self> {
        Ok(Self {
            source_file: file_path,
            module_dir: self.child_dir(module)?,
        })
    }

    /// Returns candidate source files for an out-of-line child module.
    pub fn child_file_candidates(&self, module: &ItemMod<'_>) -> Result<Vec<PathBuf>> {
        if let Some(path) = path_attr(module)? {
            return Ok(vec![
                self.module_dir.join(&path),
                self.source_dir().join(path),
            ]);
        }

        let name = module.ident.inner.as_ref();
        Ok(vec![
            self.module_dir.join(format!("{name}.rs")),
            self.module_dir.join(name).join("mod.rs"),
        ])
    }

    fn child_dir(&self, module: &ItemMod<'_>) -> Result<PathBuf> {
        if let Some(path) = path_attr(module)? {
            return Ok(self.resolve_attr_path(path));
        }

        Ok(self.module_dir.join(module.ident.inner.as_ref()))
    }

    fn resolve_attr_path(&self, path: PathBuf) -> PathBuf {
        let module_relative = self.module_dir.join(&path);
        if module_relative.exists() {
            return module_relative;
        }

        self.source_dir().join(path)
    }
}

fn path_attr(item: &ItemMod<'_>) -> MaybeResult<PathBuf> {
    let source = item.span.source_text();
    // Known semantic models intentionally omit locations and cannot declare path attributes.
    if source.is_empty() {
        return Ok(None);
    }
    let item = syn::parse_str::<syn::ItemMod>(source)
        .map_err(|e| format!("ModulePath::path_attr: failed to parse module item: {e}"))?;

    Ok(item.attrs.into_iter().find_map(|attr| {
        if !attr.path().is_ident("path") {
            return None;
        }

        let syn::Meta::NameValue(meta) = attr.meta else {
            return None;
        };
        let syn::Expr::Lit(expr) = meta.value else {
            return None;
        };
        let syn::Lit::Str(path) = expr.lit else {
            return None;
        };
        Some(PathBuf::from(path.value()))
    }))
}
