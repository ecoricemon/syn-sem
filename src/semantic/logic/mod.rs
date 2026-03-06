mod construct;
pub(crate) mod find_method;
pub(crate) mod find_ty;
mod inner;
pub mod term;
mod util;

// === Re-exports ===

pub(crate) use construct::Host;
pub use construct::ImplLogic;
pub use inner::Logic;
pub(crate) use util::{name_to_ptr, var_name};

#[cfg(test)]
pub(crate) mod test_help {
    use super::{construct::Host, inner::Logic};
    use crate::{
        etc::known,
        semantic::{
            basic_traits::{test_help::TestUsizeEvaluator, EvaluateArrayLength},
            entry::GlobalCx,
        },
        ConfigLoad, TriResult,
    };

    pub(crate) fn test_logic<'gcx>(gcx: &'gcx GlobalCx<'gcx>) -> Logic<'gcx> {
        let mut logic = Logic::new(gcx);
        let mut host = TestLogicHost::new();

        // Loads "core" then parse & make logic for it if the configuration allows.
        if gcx.get_config().load.contains(ConfigLoad::CORE) {
            let code = known::LIB_CORE_CODE;
            let file = syn::parse_str::<syn::File>(code).unwrap();
            logic.load_file(&mut host, &file).unwrap();
        }

        logic
    }

    // === TestLogicHost ===

    pub(crate) struct TestLogicHost<'gcx> {
        logic_overriding: Option<Box<dyn Host<'gcx> + 'gcx>>,
        len_evaluator: TestUsizeEvaluator,
    }

    impl<'gcx> TestLogicHost<'gcx> {
        pub(crate) fn new() -> Self {
            Self {
                logic_overriding: None,
                len_evaluator: TestUsizeEvaluator,
            }
        }

        #[allow(dead_code)]
        pub(crate) fn override_logic_host<H: Host<'gcx> + 'gcx>(&mut self, host: H) {
            self.logic_overriding = Some(Box::new(host));
        }
    }

    impl<'gcx> Host<'gcx> for TestLogicHost<'gcx> {
        #[rustfmt::skip]
        fn ident_to_npath(&mut self, ident: &syn::Ident) -> TriResult<String, ()> {
            if let Some(overriding) = &mut self.logic_overriding {
                return overriding.ident_to_npath(ident);
            }

            let ident = ident.to_string();
            if [
                "Add", "Sub", "Mul", "Div", "Rem",
                "BitXor", "BitAnd", "BitOr",
                "Shl", "Shr",
                "AddAssign", "SubAssign", "MulAssign", "DivAssign", "RemAssign",
                "BitXorAssign", "BitAndAssign", "BitOrAssign",
                "ShlAssign", "ShrAssign",
                "Deref", "Not", "Neg",
            ]
            .contains(&ident.as_str())
            {
                Ok(format!("core::ops::{ident}"))
            } else {
                Ok(ident)
            }
        }
    }

    impl<'gcx> EvaluateArrayLength<'gcx> for TestLogicHost<'gcx> {
        fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<crate::ArrayLen, ()> {
            EvaluateArrayLength::eval_array_len(&mut self.len_evaluator, expr)
        }
    }

    crate::impl_empty_method_host!(TestLogicHost<'_>);
    crate::impl_empty_scoping!(TestLogicHost<'_>);
}
