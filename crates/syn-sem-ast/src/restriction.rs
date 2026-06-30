use crate::{FromSyn, InputDesc, Path, Span, SyntaxCx};
use syn_sem_macros::CheckDropless;

/// Visibility of an item or field.
///
/// Examples include `pub`, private inherited visibility, and restricted forms like `pub(crate)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Visibility<'cx> {
    /// Unrestricted public visibility.
    Public(Span<'cx>),
    /// Restricted public visibility, such as `pub(crate)`.
    Restricted(Path<'cx>),
    /// Inherited private visibility.
    Private,
}

impl<'cx> FromSyn<'cx, syn::Visibility> for Visibility<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Visibility>) -> Self {
        match desc.input {
            syn::Visibility::Public(v) => Self::Public(desc.span(v)),
            syn::Visibility::Restricted(v) => {
                Self::Restricted(Path::from_syn(scx, desc.with_input(&v.path)))
            }
            syn::Visibility::Inherited => Self::Private,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn visibility() {
        // Proves public, inherited, and restricted visibilities are distinguished.
        type T = syn::Visibility;
        type U<'a> = Visibility<'a>;
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        // Checks unrestricted public visibility is classified as `Public`.
        // For example, `pub` has no restricting path.
        let vis = parse::<T, U>(&scx, "pub");
        assert!(matches!(&vis.value, Visibility::Public(..)));

        // Checks `pub(super)` keeps the restricting path.
        // For example, `super` becomes the single path segment.
        let vis = parse::<T, U>(&scx, "pub(super)");
        let Visibility::Restricted(path) = &vis.value else {
            panic!()
        };
        assert_eq!(&**path.get_ident().unwrap(), "super");

        // Checks `pub(crate)` keeps the crate-root restricting path.
        // For example, `crate` becomes the single path segment.
        let vis = parse::<T, U>(&scx, "pub(crate)");
        let Visibility::Restricted(path) = &vis.value else {
            panic!()
        };
        assert_eq!(&**path.get_ident().unwrap(), "crate");

        // Checks `pub(in path)` keeps the full restricting path.
        // For example, `foo::bar` preserves both path segments.
        let vis = parse::<T, U>(&scx, "pub(in foo::bar)");
        let Visibility::Restricted(path) = &vis.value else {
            panic!()
        };
        assert_eq!(&*path.segments[0].ident, "foo");
        assert_eq!(&*path.segments[1].ident, "bar");
    }
}
