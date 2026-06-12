use crate::TopCx;
use std::{collections::BTreeSet, path::PathBuf};
use syn_sem_ast as ast;
use syn_sem_common::{FilePath, Result};
use syn_sem_name::collect::{FileInput, ModulePath};

pub(crate) struct NameInputBuilder<'tcx> {
    tcx: &'tcx TopCx<'tcx>,
    seen: BTreeSet<PathBuf>,
    files: Vec<FileInput<'tcx>>,
}

impl<'tcx> NameInputBuilder<'tcx> {
    pub(crate) fn new(tcx: &'tcx TopCx<'tcx>) -> Self {
        Self {
            tcx,
            seen: BTreeSet::new(),
            files: Vec::new(),
        }
    }

    pub(crate) fn collect(mut self, entry_path: FilePath<'tcx>) -> Result<Vec<FileInput<'tcx>>> {
        let file = self.tcx.syntax.lookup_source(entry_path)?.ast();
        let path = ModulePath::from_entry_file(PathBuf::from(entry_path.as_ref()));
        self.add_file(entry_path, file);
        self.scan_file(file, &path)?;
        Ok(self.files)
    }

    fn add_file(&mut self, file_path: FilePath<'tcx>, file: &'tcx ast::File<'tcx>) -> bool {
        if !self.seen.insert(PathBuf::from(file_path.as_ref())) {
            return false;
        }
        self.files.push(FileInput { file_path, file });
        true
    }

    fn scan_file(&mut self, file: &'tcx ast::File<'tcx>, path: &ModulePath) -> Result<()> {
        for item in file.items {
            self.scan_item(item, path)?;
        }
        Ok(())
    }

    fn scan_item(&mut self, item: &'tcx ast::Item<'tcx>, path: &ModulePath) -> Result<()> {
        let ast::Item::Mod(module) = item else {
            return Ok(());
        };

        if let Some(items) = module.items {
            let path = path.enter_inline_module(module);
            for item in items {
                self.scan_item(item, &path)?;
            }
            return Ok(());
        }

        let candidates = path.child_file_candidates(module);
        let Some(file_path) = self.find_child_file(&candidates)? else {
            return Ok(());
        };
        let file = self.tcx.syntax.lookup_source(file_path)?.ast();
        if self.add_file(file_path, file) {
            let path = path.enter_external_module(module, PathBuf::from(file_path.as_ref()));
            self.scan_file(file, &path)?;
        }
        Ok(())
    }

    fn find_child_file(&self, candidates: &[PathBuf]) -> Result<Option<FilePath<'tcx>>> {
        for path in candidates {
            if let Some(file_path) = self.tcx.has_parsed(path) {
                return Ok(Some(file_path));
            }
        }

        for path in candidates {
            if path.is_file() {
                return self.tcx.read_physical_file(path).map(Some);
            }
        }

        Ok(None)
    }
}
