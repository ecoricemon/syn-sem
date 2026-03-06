use super::{
    construct::{Bound, Finder, Host, HostWrapper, ImplLogic},
    term,
};
use crate::{err, semantic::entry::GlobalCx, ExprIn, TermIn, TriResult};
use logic_eval::{Expr, Name, Term, VAR_PREFIX};

pub(crate) struct TypeFinder<'a, 'gcx, H> {
    gcx: &'gcx GlobalCx<'gcx>,
    impl_logic: &'a mut ImplLogic<'gcx>,
    host: &'a mut H,
}

impl<'a, 'gcx, H: Host<'gcx>> TypeFinder<'a, 'gcx, H> {
    pub(crate) fn new(
        gcx: &'gcx GlobalCx<'gcx>,
        impl_logic: &'a mut ImplLogic<'gcx>,
        host: &'a mut H,
    ) -> Self {
        Self {
            gcx,
            impl_logic,
            host,
        }
    }

    pub(crate) fn find_type_by_path(&mut self, path: &syn::Path) -> TriResult<TermIn<'gcx>, ()> {
        const _: () = assert!(VAR_PREFIX == '$');
        const TY_VAR: &str = "$T";

        let num_segments = path.segments.len();
        return match num_segments as u32 {
            0 => unreachable!(),
            1 => {
                // "inherent const?" - impossible.
                // "trait assoc const?" - impossible.
                err!(soft, ())
            }
            2.. => {
                // Splits the path into two parts, parent and child.
                // e.g. "a::b::C::D" -> "a::b::C" and "D"

                let parent_path = path.segments.iter().take(num_segments - 1);
                let parent = syn_path_segments_to_term(self.host, parent_path, self.gcx)?;

                let child_segment = path.segments.last().unwrap();
                let child_ident = child_segment.ident.to_string();

                let mut host = HostWrapper::new(self.gcx, self.host);
                let mut bound = Bound::new(self.gcx); // Dummy. Will be gone without being used.
                let mut finder = Finder::new(self.gcx, &mut host, &mut bound);
                let arg = finder.path_arguments_to_arg(&child_ident, &child_segment.arguments)?;
                let child = Term {
                    functor: Name::with_intern(&child_ident, self.gcx),
                    args: [arg].into(),
                };

                // Is the path "inherent const"? then returns the type of the const.
                let const_ty = Term {
                    functor: Name::with_intern(TY_VAR, self.gcx),
                    args: [].into(),
                };
                let const_id = Term {
                    functor: Name::with_intern("$_", self.gcx),
                    args: [].into(),
                };
                let query = Expr::Term(term::inher_const_4(
                    parent.clone(),
                    child.clone(),
                    const_ty.clone(),
                    const_id,
                    self.gcx,
                ));
                if let Some(found_term) = find(self.impl_logic, query) {
                    return Ok(found_term);
                };

                // Is the path "trait assoc const"? then returns the type of the const.
                let query = Expr::Term(term::assoc_const_ty_3(parent, child, const_ty, self.gcx));
                if let Some(found_term) = find(self.impl_logic, query) {
                    return Ok(found_term);
                };

                err!(soft, ())
            }
        };

        // === Internal helper functions ===

        fn find<'gcx>(
            impl_logic: &mut ImplLogic<'gcx>,
            query: ExprIn<'gcx>,
        ) -> Option<TermIn<'gcx>> {
            let mut cx = impl_logic.query(query);
            if let Some(eval) = cx.prove_next() {
                let assign = eval
                    .into_iter()
                    .find(|assign| assign.get_lhs_variable().as_ref() == TY_VAR)
                    .unwrap();
                Some(assign.rhs())
            } else {
                None
            }
        }
    }
}

/// Creates a term for the given path while dropping bound information.
pub(crate) fn syn_path_segments_to_term<'item, 'gcx, H, I>(
    host: &mut H,
    segments: I,
    gcx: &'gcx GlobalCx<'gcx>,
) -> TriResult<TermIn<'gcx>, ()>
where
    H: Host<'gcx>,
    I: ExactSizeIterator<Item = &'item syn::PathSegment> + Clone,
{
    let mut host = HostWrapper::new(gcx, host);
    let mut bound = Bound::new(gcx); // Dummy. Will be gone without being used.
    Finder::new(gcx, &mut host, &mut bound).path_segments_to_term(segments)
}
