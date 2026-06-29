use crate::Semantics;
use std::path::Path;
use syn_sem_ast::SyntaxCx;
use syn_sem_common::{CommonCx, FilePath, Result};
use syn_sem_eval::{ConstValue, EvalDb};
use syn_sem_hir::{Hir, HirBuilder};
use syn_sem_infer::{InferConstFacts, InferConstInt, InferConstValue, InferDb};
use syn_sem_name::collect::NameCollector;
use syn_sem_name::NameDb;

const MAX_ANALYSIS_PHASE_ITERATIONS: usize = 8;

/// Top-level orchestration context for extracted `syn-sem` crates.
///
/// `TopCx` owns shared infrastructure and phase contexts. Phase contexts borrow the owned common
/// context through the top-level root, keeping lower-level contexts from owning one another.
pub struct TopCx<'tcx> {
    /// Syntax parsing and source storage context.
    pub syntax: SyntaxCx<'tcx>,

    /// Shared common infrastructure context.
    //
    // `CommonCx` must be dropped last because phase contexts may hold references into it.
    pub common: Box<CommonCx>,
}

impl<'tcx> TopCx<'tcx> {
    /// Reads a physical entry file and analyzes its connected module tree.
    pub fn analyze_file(&'tcx self, entry_path: impl AsRef<Path>) -> Result<Semantics<'tcx>> {
        let entry_path = self.syntax.read_physical_file(entry_path)?;
        self.analyze_entry(entry_path)
    }

    /// Parses a virtual entry file and analyzes its connected module tree.
    pub fn analyze_virtual_file(
        &'tcx self,
        entry_path: &str,
        source_text: &str,
    ) -> Result<Semantics<'tcx>> {
        let entry_path = self.common.insert_virtual_file(entry_path, source_text)?;
        let source_text = self
            .common
            .source_text(entry_path)
            .ok_or_else(|| format!("source file is not stored: {entry_path}"))?;
        self.syntax.parse_virtual_file(entry_path, source_text)?;
        self.analyze_entry(entry_path)
    }

    fn analyze_entry(&'tcx self, entry_path: FilePath<'tcx>) -> Result<Semantics<'tcx>> {
        let name_inputs = self.syntax.collect_module_tree(entry_path)?;
        let names = NameCollector::new(name_inputs).collect(entry_path)?;
        let file = self.syntax.lookup_source(entry_path)?.ast();
        let hir = HirBuilder::new(&names).build(entry_path, file);
        let (infer, eval) = self.analyze_phases_to_fixed_point(&hir, &names)?;
        Ok(Semantics::new(self, names, hir, infer, eval))
    }

    fn analyze_phases_to_fixed_point(
        &'tcx self,
        hir: &Hir<'tcx>,
        names: &NameDb<'tcx>,
    ) -> Result<(InferDb<'tcx>, EvalDb)> {
        let mut const_facts = InferConstFacts::default();
        for _ in 0..MAX_ANALYSIS_PHASE_ITERATIONS {
            let infer = InferDb::analyze(&self.common, hir, names, &const_facts);
            let eval = EvalDb::analyze(&self.common, hir, names, &infer)?;
            let next_const_facts = self.infer_const_facts(hir, &eval);
            if next_const_facts == const_facts {
                return Ok((infer, eval));
            }
            const_facts = next_const_facts;
        }

        let infer = InferDb::analyze(&self.common, hir, names, &const_facts);
        let eval = EvalDb::analyze(&self.common, hir, names, &infer)?;
        Ok((infer, eval))
    }

    fn infer_const_facts(&self, hir: &Hir<'tcx>, eval: &EvalDb) -> InferConstFacts {
        let mut facts = InferConstFacts::default();
        for expr in hir.exprs() {
            if let Some(value) = eval
                .value_for_hir_expr(expr.id)
                .and_then(Self::infer_const_value)
            {
                facts.insert_expr_value(expr.id, value);
            }
        }
        for item in hir.items() {
            let syn_sem_hir::ItemKind::Const { .. } = item.kind else {
                continue;
            };
            let Some(def) = item.def else {
                continue;
            };
            if let Some(value) = eval
                .value_for_const_def(def)
                .and_then(Self::infer_const_value)
            {
                facts.insert_def_value(def, value);
            }
        }
        facts
    }

    fn infer_const_value(value: ConstValue) -> Option<InferConstValue> {
        match value {
            ConstValue::Int(value) => Some(InferConstValue::Int(InferConstInt {
                value: value.value,
                primitive: value.primitive,
            })),
            ConstValue::Bool(value) => Some(InferConstValue::Bool(value)),
            ConstValue::Float(_) => None,
        }
    }
}

impl<'tcx> Default for TopCx<'tcx> {
    /// Creates a top-level context with owned shared common infrastructure.
    fn default() -> Self {
        let common = Box::new(CommonCx::default());
        // Safety: `SyntaxCx` borrows the boxed common context. The allocation remains stable when
        // `TopCx` moves.
        let ccx_ref = unsafe { std::mem::transmute::<&CommonCx, &'tcx CommonCx>(common.as_ref()) };
        Self {
            syntax: SyntaxCx::new(ccx_ref),
            common,
        }
    }
}
