use crate::SyntaxCx;
use any_intern::Interned;
use num_traits::ToPrimitive;
use std::ops::Deref;
use syn::punctuated::Punctuated;
use syn_locator::Locate;
use syn_sem_macros::CheckDropless;

pub trait FromSyn<'scx, Input: ?Sized>: 'scx {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, Input>) -> Self;
}

/// Input passed while converting from `syn` into this AST.
///
/// For example, converting a parsed `syn::ItemStruct` receives the source file path and the
/// borrowed `syn` node here.
pub struct InputDesc<'a, Input: ?Sized> {
    pub file_path: &'a str,
    pub input: &'a Input,
}

impl<'scx, U: FromSyn<'scx, T>, T> FromSyn<'scx, [T]> for &'scx [U] {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, [T]>) -> Self {
        let len = desc.input.len();
        let mut items = desc.input.iter();
        scx.alloc_slice(len, |_| {
            let t = items.next().unwrap();
            U::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: t,
                },
            )
        })
    }
}

impl<'scx, U: FromSyn<'scx, T>, T> FromSyn<'scx, Vec<T>> for &'scx [U] {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, Vec<T>>) -> Self {
        Self::from_syn(
            scx,
            InputDesc {
                file_path: desc.file_path,
                input: desc.input.as_slice(),
            },
        )
    }
}

impl<'scx, U: FromSyn<'scx, T>, T, P> FromSyn<'scx, Punctuated<T, P>> for &'scx [U] {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, Punctuated<T, P>>) -> Self {
        let len = desc.input.len();
        let mut items = desc.input.into_iter();
        scx.alloc_slice(len, |_| {
            let t = items.next().unwrap();
            U::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: t,
                },
            )
        })
    }
}

impl<'scx, U: FromSyn<'scx, T>, T> FromSyn<'scx, Option<T>> for Option<U> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, Option<T>>) -> Self {
        desc.input.as_ref().map(|t| {
            U::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: t,
                },
            )
        })
    }
}

/// An identifier in source code.
///
/// Examples include `foo`, `Self`, or a synthesized tuple-field name like `0`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Ident<'scx> {
    pub inner: Interned<'scx, str>,
    pub span: Span<'scx>,
}

impl<'scx> Ident<'scx> {
    pub fn empty(scx: &'scx SyntaxCx) -> Self {
        Self::from_str(scx, "", Span::empty())
    }

    pub fn from_str(scx: &'scx SyntaxCx, value: &str, span: Span<'scx>) -> Self {
        Self {
            inner: scx.intern(value),
            span,
        }
    }

    pub fn from_number<T: ToPrimitive>(scx: &'scx SyntaxCx, value: T, span: Span<'scx>) -> Self {
        let value = value.to_u64().unwrap();
        Self {
            inner: scx.intern_formatted_str(&value, (value % 10 + 1) as usize),
            span,
        }
    }
}

impl<'scx> FromSyn<'scx, syn::Ident> for Ident<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Ident>) -> Self {
        Self {
            inner: scx.intern(&desc.input.to_string()),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

impl<'scx> FromSyn<'scx, syn::Token![self]> for Ident<'scx> {
    fn from_syn(scx: &'scx SyntaxCx, desc: InputDesc<'_, syn::Token![self]>) -> Self {
        Self {
            inner: scx.intern("self"),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

impl<'scx> Deref for Ident<'scx> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// A signed integer value with source span information.
///
/// For example, tuple-field indexes can be represented as source-like numeric identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Isize<'scx> {
    pub value: isize,
    pub span: Span<'scx>,
}

/// A byte range into the original source text.
///
/// For example, the span for `foo` in `let foo = 1;` points back to exactly that identifier text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Span<'scx> {
    /// The whole source text.
    //
    // We do not intern about the whole text.
    text: &'scx str,
    start: u32,
    end: u32,
}

impl<'scx> Span<'scx> {
    pub fn empty() -> Self {
        Self {
            text: "",
            start: 0,
            end: 0,
        }
    }

    pub fn from_locatable<T: Locate>(scx: &'scx SyntaxCx, file_path: &str, item: &T) -> Self {
        let source = scx.get_source(file_path).unwrap();
        let loc = item.location(source.locator());
        let text = &*source.text;

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
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn test_ident() {
        // Proves `Ident` preserves the parsed identifier text.
        let scx = SyntaxCx::default();

        // Non-empty ident
        let ident = parse::<syn::Ident, Ident>(&scx, "A");
        assert_eq!(&*ident, "A");
    }
}
