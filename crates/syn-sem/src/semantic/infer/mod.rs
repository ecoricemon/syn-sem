mod proc;
mod ty;

// === Re-exports ===

pub(crate) use proc::{Host, Inferable, Inferer};
pub(crate) use ty::{
    InferArrayLen, Param, Type, TypeArray, TypeId, TypeMut, TypeNamed, TypeRef, TypeScalar,
    TypeTuple, UniqueTypes,
};

#[cfg(test)]
pub(crate) mod test_help {
    use super::{
        proc::{Host, Inferer},
        ty::{Type, TypeNamed, UniqueTypes},
    };
    use crate::{
        etc::syn::SynPath,
        semantic::{
            basic_traits::{test_help::TestUsizeEvaluator, EvaluateArrayLength},
            entry::GlobalCx,
            logic::{self, test_help::TestLogicHost},
        },
        Intern, TriResult,
    };
    use syn_locator::Locate;

    pub(crate) fn test_inferer<'gcx>(gcx: &'gcx GlobalCx<'gcx>) -> Inferer<'gcx> {
        Inferer::new(gcx)
    }

    // === TestInferLogicHost ===

    pub(crate) struct TestInferLogicHost<'gcx> {
        gcx: &'gcx GlobalCx<'gcx>,
        infer_overriding: Option<Box<dyn Host<'gcx> + 'gcx>>,
        logic_host: TestLogicHost<'gcx>,
        len_evaluator: TestUsizeEvaluator,
    }

    impl<'gcx> TestInferLogicHost<'gcx> {
        pub(crate) fn new(gcx: &'gcx GlobalCx<'gcx>) -> Self {
            Self {
                gcx,
                infer_overriding: None,
                logic_host: TestLogicHost::new(),
                len_evaluator: TestUsizeEvaluator,
            }
        }

        #[allow(dead_code)]
        pub(crate) fn override_infer_host<H: Host<'gcx> + 'gcx>(&mut self, host: H) {
            self.infer_overriding = Some(Box::new(host));
        }

        #[allow(dead_code)]
        pub(crate) fn override_logic_host<H: logic::Host<'gcx> + 'gcx>(&mut self, host: H) {
            self.logic_host.override_logic_host(host);
        }
    }

    impl<'gcx> Host<'gcx> for TestInferLogicHost<'gcx> {
        fn syn_path_to_type(
            &mut self,
            syn_path: SynPath,
            types: &mut UniqueTypes<'gcx>,
        ) -> TriResult<Type<'gcx>, ()> {
            if let Some(overriding) = &mut self.infer_overriding {
                return overriding.syn_path_to_type(syn_path, types);
            }

            let res = Type::Named(TypeNamed {
                name: self.gcx.intern_str(&syn_path.path.code()),
                params: [].into(),
            });
            Ok(res)
        }
    }

    impl<'gcx> logic::Host<'gcx> for TestInferLogicHost<'gcx> {
        fn ident_to_npath(&mut self, ident: &syn::Ident) -> TriResult<String, ()> {
            self.logic_host.ident_to_npath(ident)
        }
    }

    impl<'gcx> EvaluateArrayLength<'gcx> for TestInferLogicHost<'gcx> {
        fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<crate::ArrayLen, ()> {
            EvaluateArrayLength::eval_array_len(&mut self.len_evaluator, expr)
        }
    }

    crate::impl_empty_method_host!(TestInferLogicHost<'_>);
    crate::impl_empty_scoping!(TestInferLogicHost<'_>);
}
