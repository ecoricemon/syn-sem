use crate::SyntaxContext;
use any_intern::Interned;
use std::{mem, ops::Deref, path::Path};
use syn::punctuated::Punctuated;
use syn_locator::Locate;

pub trait FromSyn<'scx, Input: ?Sized>: 'scx {
    fn from_syn(scx: &'scx SyntaxContext, input: &Input) -> Self;
}

impl<'scx, U: FromSyn<'scx, T>, T> FromSyn<'scx, [T]> for &'scx [U] {
    fn from_syn(scx: &'scx SyntaxContext, input: &[T]) -> Self {
        let len = input.len();
        let mut items = input.iter();
        scx.alloc_slice(len, |_| {
            let t = items.next().unwrap();
            U::from_syn(scx, t)
        })
    }
}

impl<'scx, U: FromSyn<'scx, T>, T, P> FromSyn<'scx, Punctuated<T, P>> for &'scx [U] {
    fn from_syn(scx: &'scx SyntaxContext, input: &Punctuated<T, P>) -> Self {
        let len = input.len();
        let mut items = input.into_iter();
        scx.alloc_slice(len, |_| {
            let t = items.next().unwrap();
            U::from_syn(scx, t)
        })
    }
}

impl<'scx, U: FromSyn<'scx, T>, T> FromSyn<'scx, Option<T>> for Option<U> {
    fn from_syn(scx: &'scx SyntaxContext, input: &Option<T>) -> Self {
        input.as_ref().map(|t| U::from_syn(scx, t))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ident<'scx> {
    pub inner: Interned<'scx, str>,
    pub span: Span<'scx>,
}

impl<'scx> Ident<'scx> {
    pub fn empty(scx: &'scx SyntaxContext) -> Self {
        Self {
            inner: scx.intern(""),
            span: Span::undefined(),
        }
    }

    pub fn from_usize(scx: &'scx SyntaxContext, value: usize, span: Span<'scx>) -> Self {
        Self {
            inner: scx.intern_formatted_str(&value, value % 10 + 1),
            span,
        }
    }

    pub fn from_u32(scx: &'scx SyntaxContext, value: u32, span: Span<'scx>) -> Self {
        Self {
            inner: scx.intern_formatted_str(&value, (value % 10 + 1) as usize),
            span,
        }
    }
}

impl<'scx> FromSyn<'scx, syn::Ident> for Ident<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::Ident) -> Self {
        Self {
            inner: scx.intern(&input.to_string()),
            span: Span::from_locatable(scx, input),
        }
    }
}

impl<'scx> Deref for Ident<'scx> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Span<'scx> {
    pub text: &'scx str,
    pub start: u32,
    pub end: u32,
}

impl<'scx> Span<'scx> {
    pub fn undefined() -> Self {
        Self {
            text: "",
            start: 0,
            end: 0,
        }
    }

    pub fn from_locatable<T: Locate>(scx: &'scx SyntaxContext, value: &T) -> Self {
        let loc = value.location();
        let file_path = &*loc.file_path;
        let source = scx.get_source(Path::new(file_path));
        let text = &*source.text;

        // Change lifetime to 'scx.
        // Safety: `Span<'scx>` cannot outlive the `SyntaxContext`. In other words, the text inside
        // `SyntaxContext` is guaranteed to be valid while `Span<'scx>` is alive.
        let text = unsafe { mem::transmute::<&str, &str>(text) };

        Self {
            text,
            start: loc.start as u32,
            end: loc.end as u32,
        }
    }

    pub fn source_text(&self) -> &'scx str {
        &self.text[self.start as usize..self.end as usize]
    }
}

impl Default for Span<'_> {
    fn default() -> Self {
        Self::undefined()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn test_ident() {
        let scx = create_context();

        // Non-empty ident
        let ident = parse::<syn::Ident, Ident>(&scx, "A");
        assert_eq!(&*ident, "A");
    }
}
