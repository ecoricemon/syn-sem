use crate::{semantic::tree::ArrayLen, TriResult};

// === Scoping ===

pub(crate) trait Scoping {
    fn on_enter_scope(&mut self, scope: Scope);
    fn on_exit_scope(&mut self, scope: Scope);
}

#[macro_export]
macro_rules! impl_empty_scoping {
    ($ty:ty) => {
        impl $crate::semantic::basic_traits::Scoping for $ty {
            fn on_enter_scope(&mut self, _: $crate::semantic::basic_traits::Scope) {}
            fn on_exit_scope(&mut self, _: $crate::semantic::basic_traits::Scope) {}
        }
    };
}

#[derive(Clone, Copy)]
pub(crate) enum Scope<'a> {
    Mod(&'a syn::ItemMod),
    ItemFn(&'a syn::ItemFn),
    Block(&'a syn::Block),
}

impl Scope<'_> {
    pub(crate) fn from_raw(raw: RawScope) -> Self {
        unsafe {
            match raw {
                RawScope::Mod(ptr) => Self::Mod(ptr.as_ref().unwrap()),
                RawScope::ItemFn(ptr) => Self::ItemFn(ptr.as_ref().unwrap()),
                RawScope::Block(ptr) => Self::Block(ptr.as_ref().unwrap()),
            }
        }
    }

    pub(crate) fn into_raw(self) -> RawScope {
        match self {
            Self::Mod(ref_) => RawScope::Mod(ref_ as *const _),
            Self::ItemFn(ref_) => RawScope::ItemFn(ref_ as *const _),
            Self::Block(ref_) => RawScope::Block(ref_ as *const _),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum RawScope {
    Mod(*const syn::ItemMod),
    ItemFn(*const syn::ItemFn),
    Block(*const syn::Block),
}

// === EvaluateArrayLength ===

pub(crate) trait EvaluateArrayLength<'gcx> {
    fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<ArrayLen, ()>;
}

#[cfg(test)]
pub(crate) mod test_help {
    use super::EvaluateArrayLength;
    use crate::{err, semantic::tree::ArrayLen, TriResult};

    pub(crate) struct TestUsizeEvaluator;

    impl EvaluateArrayLength<'static> for TestUsizeEvaluator {
        fn eval_array_len(&mut self, expr: &syn::Expr) -> TriResult<ArrayLen, ()> {
            let syn::Expr::Lit(expr_lit) = expr else {
                return err!(hard, "test logic host cannot evaluate complex exprs");
            };
            let syn::Lit::Int(int) = &expr_lit.lit else {
                return err!(hard, "test logic host cannot evaluate complex exprs");
            };
            match int.base10_parse() {
                Ok(n) => Ok(ArrayLen::Fixed(n)),
                Err(e) => err!(hard, "{e:?}"),
            }
        }
    }
}
