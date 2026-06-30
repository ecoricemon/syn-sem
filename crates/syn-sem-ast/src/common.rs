use crate::SyntaxCx;
use any_intern::Interned;
use num_traits::ToPrimitive;
use std::{
    fmt::{self, Display},
    ops::Deref,
};
use syn::punctuated::Punctuated;
use syn_locator::{Locate, Locator};
use syn_sem_common::{FilePath, SourceText};
use syn_sem_macros::CheckDropless;

/// Converts a `syn` syntax node into the semantic AST representation.
pub trait FromSyn<'cx, Input: ?Sized>: 'cx {
    /// Builds `Self` from a borrowed `syn` input and conversion context.
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, Input>) -> Self;
}

/// Input passed while converting from `syn` into this AST.
///
/// For example, converting a parsed `syn::ItemStruct` receives the source file path and the
/// borrowed `syn` node here.
pub struct InputDesc<'cx, 'syn, Input: ?Sized> {
    /// Interned path of the source file that owns `input`.
    pub file_path: FilePath<'cx>,
    /// Interned source text that owns `input`.
    pub source_text: SourceText<'cx>,
    /// Locator populated for the parsed source that owns `input`.
    pub locator: &'syn Locator,
    /// Borrowed syntax node being converted.
    pub input: &'syn Input,
}

impl<'cx, 'syn, Input: ?Sized> Clone for InputDesc<'cx, 'syn, Input> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'cx, 'syn, Input: ?Sized> Copy for InputDesc<'cx, 'syn, Input> {}

impl<'cx, 'syn, Input: ?Sized> InputDesc<'cx, 'syn, Input> {
    /// Returns an input descriptor for another node in the same source.
    pub fn with_input<NewInput: ?Sized>(
        self,
        input: &'syn NewInput,
    ) -> InputDesc<'cx, 'syn, NewInput> {
        InputDesc {
            file_path: self.file_path,
            source_text: self.source_text,
            locator: self.locator,
            input,
        }
    }

    /// Creates a span for `item` in this descriptor's source.
    pub fn span<T: Locate + ?Sized>(self, item: &T) -> Span<'cx> {
        Span::from_locatable(self.source_text, self.locator, item)
    }
}

impl<'cx, U: FromSyn<'cx, T>, T> FromSyn<'cx, [T]> for &'cx [U] {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, [T]>) -> Self {
        let len = desc.input.len();
        let mut items = desc.input.iter();
        scx.alloc_slice(len, |_| {
            let t = items.next().unwrap();
            U::from_syn(scx, desc.with_input(t))
        })
    }
}

impl<'cx, U: FromSyn<'cx, T>, T> FromSyn<'cx, Vec<T>> for &'cx [U] {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, Vec<T>>) -> Self {
        Self::from_syn(scx, desc.with_input(desc.input.as_slice()))
    }
}

impl<'cx, U: FromSyn<'cx, T>, T, P> FromSyn<'cx, Punctuated<T, P>> for &'cx [U] {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, Punctuated<T, P>>) -> Self {
        let len = desc.input.len();
        let mut items = desc.input.into_iter();
        scx.alloc_slice(len, |_| {
            let t = items.next().unwrap();
            U::from_syn(scx, desc.with_input(t))
        })
    }
}

impl<'cx, U: FromSyn<'cx, T>, T> FromSyn<'cx, Option<T>> for Option<U> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, Option<T>>) -> Self {
        desc.input
            .as_ref()
            .map(|t| U::from_syn(scx, desc.with_input(t)))
    }
}

/// An identifier in source code.
///
/// Examples include `foo`, `Self`, or a synthesized tuple-field name like `0`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Ident<'cx> {
    /// Interned identifier text.
    pub inner: Interned<'cx, str>,
    /// Source span of the identifier.
    pub span: Span<'cx>,
}

impl<'cx> Ident<'cx> {
    /// Creates an empty synthesized identifier.
    pub fn empty(scx: &'cx SyntaxCx<'cx>) -> Self {
        Self::from_str(scx, "", Span::new_empty(scx))
    }

    /// Creates an identifier by interning `value`.
    pub fn from_str(scx: &'cx SyntaxCx<'cx>, value: &str, span: Span<'cx>) -> Self {
        Self {
            inner: scx.intern(value),
            span,
        }
    }

    /// Creates a numeric synthesized identifier.
    pub fn from_number<T: ToPrimitive>(scx: &'cx SyntaxCx<'cx>, value: T, span: Span<'cx>) -> Self {
        let value = value.to_u64().unwrap();
        Self {
            inner: scx
                .intern_display(&value, (value % 10 + 1) as usize)
                .unwrap(),
            span,
        }
    }
}

impl<'cx> FromSyn<'cx, syn::Ident> for Ident<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Ident>) -> Self {
        Self {
            inner: scx.intern(&desc.input.to_string()),
            span: desc.span(desc.input),
        }
    }
}

impl<'cx> FromSyn<'cx, syn::Token![self]> for Ident<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, '_, syn::Token![self]>) -> Self {
        Self {
            inner: scx.intern("self"),
            span: desc.span(desc.input),
        }
    }
}

impl Deref for Ident<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Display for Ident<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self.inner.as_ref(), f)
    }
}

/// A signed integer value with source span information.
///
/// For example, tuple-field indexes can be represented as source-like numeric identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Isize<'cx> {
    /// Numeric value.
    pub value: isize,
    /// Source span of the value.
    pub span: Span<'cx>,
}

/// A byte range into the original source text.
///
/// For example, the span for `foo` in `let foo = 1;` points back to exactly that identifier text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Span<'cx> {
    /// The whole source text.
    source_text: SourceText<'cx>,
    start: u32,
    end: u32,
}

impl<'cx> Span<'cx> {
    /// Creates an empty span in `scx`.
    pub fn new_empty(scx: &'cx SyntaxCx<'cx>) -> Self {
        Self {
            source_text: scx.intern(""),
            start: 0,
            end: 0,
        }
    }

    /// Creates a span for a `syn_locator` node in the given file.
    pub fn from_locatable<T: Locate + ?Sized>(
        source_text: SourceText<'cx>,
        locator: &Locator,
        item: &T,
    ) -> Self {
        let loc = item.location(locator);

        Self {
            source_text,
            start: loc.start as u32,
            end: loc.end as u32,
        }
    }

    /// Returns the source text covered by this span.
    pub fn source_text(&self) -> &str {
        &self.source_text.as_ref()[self.start as usize..self.end as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn ident() {
        // Proves `Ident` preserves the parsed identifier text.
        let ccx = syn_sem_common::CommonCx::default();
        let scx = SyntaxCx::new(&ccx);

        // Checks a non-empty identifier keeps its parsed spelling.
        // For example, `A` stays available through the interned `Ident`.
        let ident = parse::<syn::Ident, Ident>(&scx, "A");
        assert_eq!(&**ident, "A");
    }
}
