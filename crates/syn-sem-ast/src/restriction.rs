use crate::{FromSyn, Path, Span, SyntaxContext};
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Visibility<'scx> {
    Public(Span<'scx>),
    PublicPath(Path<'scx>),
    Private,
}

impl<'scx> FromSyn<'scx, syn::Visibility> for Visibility<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::Visibility) -> Self {
        match input {
            syn::Visibility::Public(v) => Self::Public(Span::from_locatable(scx, v)),
            syn::Visibility::Restricted(v) => Self::PublicPath(Path::from_syn(scx, &v.path)),
            syn::Visibility::Inherited => Self::Private,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn test_visibility() {
        type T = syn::Visibility;
        type U<'a> = Visibility<'a>;
        let scx = create_context();

        // Public visibility
        let vis = parse::<T, U>(&scx, "pub");
        assert!(matches!(vis, Visibility::Public(..)));

        // Restricted visibility - pub(super)
        let vis = parse::<T, U>(&scx, "pub(super)");
        let Visibility::PublicPath(path) = vis else {
            panic!()
        };
        assert_eq!(&**path.get_ident().unwrap(), "super");

        // Restricted visibility - pub(super)
        let vis = parse::<T, U>(&scx, "pub(crate)");
        let Visibility::PublicPath(path) = vis else {
            panic!()
        };
        assert_eq!(&**path.get_ident().unwrap(), "crate");

        // Restricted visibility - pub(in path)
        let vis = parse::<T, U>(&scx, "pub(in foo::bar)");
        let Visibility::PublicPath(path) = vis else {
            panic!()
        };
        assert_eq!(&*path.segments[0].ident, "foo");
        assert_eq!(&*path.segments[1].ident, "bar");
    }
}
