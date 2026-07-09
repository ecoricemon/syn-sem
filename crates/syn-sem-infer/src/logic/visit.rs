//! Shared helpers for logic term handling.

use super::{Atom, Term};

/// Visits each left-side variable and the corresponding right-side term when the two terms are
/// shape-compatible.
///
/// Returns `true` after visiting compatible terms, or `false` without calling `visit` when their
/// shapes are incompatible.
pub(crate) fn visit_left_var<'cx, 'right, F>(
    left: &Term<'cx>,
    right: &'right Term<'cx>,
    visit: &mut F,
) -> bool
where
    F: FnMut(Atom<'cx>, &'right Term<'cx>),
{
    if !left.is_shape_compatible_with(right) {
        return false;
    }
    visit_left_var_unchecked(left, right, visit);
    true
}

fn visit_left_var_unchecked<'cx, 'right, F>(
    left: &Term<'cx>,
    right: &'right Term<'cx>,
    visit: &mut F,
) where
    F: FnMut(Atom<'cx>, &'right Term<'cx>),
{
    if left.is_variable() {
        visit(left.functor, right);
        return;
    }
    if right.is_variable() {
        return;
    }

    debug_assert_eq!(left.functor, right.functor);
    debug_assert_eq!(left.args.len(), right.args.len());

    for (left_arg, right_arg) in left.args.iter().zip(&right.args) {
        visit_left_var_unchecked(left_arg, right_arg, visit);
    }
}
