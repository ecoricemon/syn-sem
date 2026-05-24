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
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Visibility>) -> Self {
        match desc.input {
            syn::Visibility::Public(v) => {
                Self::Public(Span::from_locatable(scx, desc.file_path, v))
            }
            syn::Visibility::Restricted(v) => Self::Restricted(Path::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &v.path,
                },
            )),
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
        let ccx = syn_sem_common::CommonCx::new();
        let cx = SyntaxCx::new(&ccx);

        // Public visibility
        let vis = parse::<T, U>(&cx, "pub");
        assert!(matches!(vis, Visibility::Public(..)));

        // Restricted visibility - pub(super)
        let vis = parse::<T, U>(&cx, "pub(super)");
        let Visibility::Restricted(path) = vis else {
            panic!()
        };
        assert_eq!(&**path.get_ident().unwrap(), "super");

        // Restricted visibility - pub(super)
        let vis = parse::<T, U>(&cx, "pub(crate)");
        let Visibility::Restricted(path) = vis else {
            panic!()
        };
        assert_eq!(&**path.get_ident().unwrap(), "crate");

        // Restricted visibility - pub(in path)
        let vis = parse::<T, U>(&cx, "pub(in foo::bar)");
        let Visibility::Restricted(path) = vis else {
            panic!()
        };
        assert_eq!(&*path.segments[0].ident, "foo");
        assert_eq!(&*path.segments[1].ident, "bar");
    }
}
