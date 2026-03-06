pub(crate) mod context;

pub use context::{Config, ConfigLoad, GlobalCx};

use crate::{
    semantic::analyze::{Analyzer, Semantics},
    Result,
};
use std::mem;

#[derive(Debug, Default)]
pub struct AnalysisSession<'gcx> {
    gcx: GlobalCx<'gcx>,
}

impl<'gcx> AnalysisSession<'gcx> {
    pub fn run<F>(self, f: F) -> Result<Analyzed<'gcx>>
    where
        F: FnOnce(Analyzer<'gcx>) -> Result<Semantics<'gcx>>,
    {
        // Changes lifetime
        let gcx: &'gcx GlobalCx<'gcx> = unsafe { mem::transmute(&self.gcx) };

        let analyzer = Analyzer::new(gcx);
        let sem = f(analyzer)?;
        Ok(Analyzed { gcx: self.gcx, sem })
    }
}

#[derive(Debug)]
pub struct Analyzed<'gcx> {
    pub gcx: GlobalCx<'gcx>,
    pub sem: Semantics<'gcx>,
}
