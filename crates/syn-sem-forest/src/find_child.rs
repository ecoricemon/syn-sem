use crate::SynId;
use std::any::TypeId;

pub trait FindChild {
    /// Visits all descendants having the given types.
    fn visit_descendant<F: FnMut(usize, SynId)>(&self, descendant_types: &[TypeId], f: &mut F);
}

impl<T: FindChild> FindChild for Vec<T> {
    fn visit_descendant<F: FnMut(usize, SynId)>(&self, descendant_types: &[TypeId], f: &mut F) {
        for elem in self {
            elem.visit_descendant(descendant_types, f);
        }
    }
}

impl<T: FindChild, P> FindChild for syn::punctuated::Punctuated<T, P> {
    fn visit_descendant<F: FnMut(usize, SynId)>(&self, descendant_types: &[TypeId], f: &mut F) {
        for elem in self {
            elem.visit_descendant(descendant_types, f);
        }
    }
}

impl<T: FindChild> FindChild for Option<T> {
    fn visit_descendant<F: FnMut(usize, SynId)>(&self, descendant_types: &[TypeId], f: &mut F) {
        if let Some(inner) = self {
            inner.visit_descendant(descendant_types, f);
        }
    }
}

impl<T: FindChild> FindChild for Box<T> {
    fn visit_descendant<F: FnMut(usize, SynId)>(&self, descendant_types: &[TypeId], f: &mut F) {
        (**self).visit_descendant(descendant_types, f);
    }
}

impl<T0, T1> FindChild for (T0, T1)
where
    T0: FindChild,
    T1: FindChild,
{
    fn visit_descendant<F: FnMut(usize, SynId)>(&self, descendant_types: &[TypeId], f: &mut F) {
        self.0.visit_descendant(descendant_types, f);
        self.1.visit_descendant(descendant_types, f);
    }
}

impl<T0, T1, T2> FindChild for (T0, T1, T2)
where
    T0: FindChild,
    T1: FindChild,
    T2: FindChild,
{
    fn visit_descendant<F: FnMut(usize, SynId)>(&self, descendant_types: &[TypeId], f: &mut F) {
        self.0.visit_descendant(descendant_types, f);
        self.1.visit_descendant(descendant_types, f);
        self.2.visit_descendant(descendant_types, f);
    }
}
