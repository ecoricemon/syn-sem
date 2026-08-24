use crate::Semantics;
use std::path::Path;
use syn_sem_ast::{SourceKind, SyntaxCx};
use syn_sem_common::{CommonCx, Result, Str};
use syn_sem_eval::{ConstValue, EvalDb, EvalPlan};
use syn_sem_hir::Hir;
use syn_sem_infer::{InferConstFacts, InferConstValue, InferDb};
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
    /// Stores virtual source text, parses it, and returns its interned file path.
    pub fn add_virtual_file(&'tcx self, file_path: &str, source_text: &str) -> Result<Str<'tcx>> {
        let file_path = self.common.insert_virtual_file(file_path, source_text)?;
        let source_text = self
            .common
            .source_text(file_path)
            .ok_or_else(|| format!("source file is not stored: {file_path}"))?;
        self.syntax
            .parse_file(file_path, source_text, SourceKind::Virtual)?;
        Ok(file_path)
    }

    /// Reads a physical entry file and analyzes its connected module tree.
    pub fn analyze_file(&'tcx self, entry_path: impl AsRef<Path>) -> Result<Semantics<'tcx>> {
        let entry_path = self.syntax.read_physical_file(entry_path)?;
        self.analyze_entry(entry_path)
    }

    /// Analyzes a stored virtual entry file and its connected module tree.
    pub fn analyze_virtual_file(&'tcx self, entry_path: &str) -> Result<Semantics<'tcx>> {
        let entry_path = self.common.intern_path(Path::new(entry_path));
        if !self.syntax.has_source(entry_path) {
            return Err(format!("virtual source file is not stored: {entry_path}").into());
        }
        self.analyze_entry(entry_path)
    }

    fn analyze_entry(&'tcx self, entry_path: Str<'tcx>) -> Result<Semantics<'tcx>> {
        let name_inputs = self.syntax.collect_module_tree(entry_path)?;
        let names = NameDb::build(name_inputs.iter().copied(), [entry_path])?;
        let hir = Hir::build_module_tree(&names, name_inputs, [entry_path])?;
        let (infer, eval) = self.analyze_phases_to_fixed_point(&hir, &names)?;
        Ok(Semantics::new(self, names, hir, infer, eval))
    }

    fn analyze_phases_to_fixed_point(
        &'tcx self,
        hir: &Hir<'tcx>,
        names: &NameDb<'tcx>,
    ) -> Result<(InferDb<'tcx>, EvalDb)> {
        let eval_plan = EvalPlan::new(hir, names)?;
        run_to_fixed_point(
            InferConstFacts::default(),
            MAX_ANALYSIS_PHASE_ITERATIONS,
            |const_facts| {
                let infer = InferDb::analyze(&self.common, hir, names, const_facts);
                let eval = EvalDb::analyze(&eval_plan, hir, &infer)?;
                let next_const_facts = self.infer_const_facts(&eval);
                Ok(((infer, eval), next_const_facts))
            },
        )
    }

    fn infer_const_facts(&self, eval: &EvalDb) -> InferConstFacts {
        let mut facts = InferConstFacts::default();
        for (expr, value) in eval.hir_expr_values() {
            if let Some(value) = Self::infer_const_value(value) {
                facts.insert_expr_value(expr, value);
            }
        }
        for (def, value) in eval.const_def_values() {
            if let Some(value) = Self::infer_const_value(value) {
                facts.insert_def_value(def, value);
            }
        }
        facts
    }

    fn infer_const_value(value: ConstValue) -> Option<InferConstValue> {
        match value {
            ConstValue::Int(value) => Some(InferConstValue::Int(value)),
            ConstValue::Bool(value) => Some(InferConstValue::Bool(value)),
            ConstValue::Float(_) => None,
        }
    }
}

fn run_to_fixed_point<State, Output>(
    mut state: State,
    max_iterations: usize,
    mut step: impl FnMut(&State) -> Result<(Output, State)>,
) -> Result<Output>
where
    State: PartialEq,
{
    for _ in 0..max_iterations {
        let (output, next_state) = step(&state)?;
        if next_state == state {
            return Ok(output);
        }
        state = next_state;
    }

    Err(format!("semantic analysis did not converge after {max_iterations} iterations").into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_point_returns_the_output_from_the_converged_iteration() {
        let output = run_to_fixed_point(0, 4, |state| {
            let next = if *state == 0 { 1 } else { *state };
            Ok((*state, next))
        })
        .unwrap();

        assert_eq!(output, 1);
    }

    #[test]
    fn fixed_point_reports_non_convergence_at_the_iteration_limit() {
        let err = run_to_fixed_point(0, 4, |state| Ok((*state, 1 - *state)))
            .expect_err("oscillating state should not converge");

        assert_eq!(
            err.to_string(),
            "semantic analysis did not converge after 4 iterations"
        );
    }
}
