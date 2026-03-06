#[derive(Debug, Clone)]
pub(crate) struct SynPath<'a> {
    // Allow dead code for future use
    #[allow(dead_code)]
    pub(crate) kind: SynPathKind,
    pub(crate) qself: Option<&'a syn::QSelf>,
    pub(crate) path: &'a syn::Path,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SynPathKind {
    Type,
    Expr,
    Pat,
}
