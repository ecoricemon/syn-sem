mod proc;
mod value;

// Re-exports
pub use proc::Evaluated;
pub(crate) use proc::{Evaluator, Host};
pub use value::{ConstGeneric, Enum, Field, Fn, Scalar, Value};

#[cfg(test)]
pub(crate) mod test_help {
    use super::{
        proc::Host,
        value::{Fn, Value},
    };
    use crate::{
        etc::syn::SynPath,
        semantic::infer::{self, Inferer},
        TriResult,
    };
    use logic_eval_util::str::StrPath;

    pub(crate) struct TestEvalHost<'a, 'gcx> {
        inferer: &'a Inferer<'gcx>,
    }

    impl<'a, 'gcx> TestEvalHost<'a, 'gcx> {
        pub(crate) fn new(inferer: &'a Inferer<'gcx>) -> Self {
            Self { inferer }
        }
    }

    impl<'gcx> Host<'gcx> for TestEvalHost<'_, 'gcx> {
        fn find_type(&mut self, expr: &syn::Expr) -> TriResult<infer::Type<'gcx>, ()> {
            let ty = self.inferer.get_type(expr).unwrap().clone();
            Ok(ty)
        }

        fn find_fn(&mut self, _name: StrPath, _types: &[infer::Type<'gcx>]) -> Fn {
            panic!()
        }

        fn syn_path_to_value(&mut self, _syn_path: SynPath) -> TriResult<Value<'gcx>, ()> {
            panic!()
        }
    }

    crate::impl_empty_scoping!(TestEvalHost<'_, '_>);
}
