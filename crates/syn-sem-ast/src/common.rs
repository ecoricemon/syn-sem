use crate::SyntaxCx;
use any_intern::Interned;
use num_traits::ToPrimitive;
use std::ops::Deref;
use syn::punctuated::Punctuated;
use syn_locator::Locate;
use syn_sem_common::FilePath;
use syn_sem_macros::CheckDropless;

/// Converts a `syn` syntax node into the semantic AST representation.
pub trait FromSyn<'cx, Input: ?Sized>: 'cx {
    /// Builds `Self` from a borrowed `syn` input and conversion context.
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, Input>) -> Self;
}

/// Input passed while converting from `syn` into this AST.
///
/// For example, converting a parsed `syn::ItemStruct` receives the source file path and the
/// borrowed `syn` node here.
pub struct InputDesc<'cx, Input: ?Sized> {
    /// Interned path of the source file that owns `input`.
    pub file_path: FilePath<'cx>,
    /// Borrowed syntax node being converted.
    pub input: &'cx Input,
}

impl<'cx, U: FromSyn<'cx, T>, T> FromSyn<'cx, [T]> for &'cx [U] {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, [T]>) -> Self {
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

impl<'cx, U: FromSyn<'cx, T>, T> FromSyn<'cx, Vec<T>> for &'cx [U] {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, Vec<T>>) -> Self {
        Self::from_syn(
            scx,
            InputDesc {
                file_path: desc.file_path,
                input: desc.input.as_slice(),
            },
        )
    }
}

impl<'cx, U: FromSyn<'cx, T>, T, P> FromSyn<'cx, Punctuated<T, P>> for &'cx [U] {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, Punctuated<T, P>>) -> Self {
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

impl<'cx, U: FromSyn<'cx, T>, T> FromSyn<'cx, Option<T>> for Option<U> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, Option<T>>) -> Self {
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
pub struct Ident<'cx> {
    /// Interned identifier text.
    pub inner: Interned<'cx, str>,
    /// Source span of the identifier.
    pub span: Span<'cx>,
}

impl<'cx> Ident<'cx> {
    /// Creates an empty synthesized identifier.
    pub fn empty(scx: &'cx SyntaxCx<'cx>) -> Self {
        Self::from_str(scx, "", Span::empty())
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
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Ident>) -> Self {
        Self {
            inner: scx.intern(&desc.input.to_string()),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

impl<'cx> FromSyn<'cx, syn::Token![self]> for Ident<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Token![self]>) -> Self {
        Self {
            inner: scx.intern("self"),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

impl<'cx> Deref for Ident<'cx> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
    //
    // We do not intern about the whole text.
    text: &'cx str,
    start: u32,
    end: u32,
}

impl<'cx> Span<'cx> {
    /// Creates an empty span.
    pub fn empty() -> Self {
        Self {
            text: "",
            start: 0,
            end: 0,
        }
    }

    /// Creates a span for a `syn_locator` node in the given file.
    pub fn from_locatable<T: Locate>(
        scx: &'cx SyntaxCx<'cx>,
        file_path: FilePath<'cx>,
        item: &T,
    ) -> Self {
        let source = scx.get_source(file_path).unwrap();
        let loc = item.location(source.locator());
        let text = source.text.as_ref();

        Self {
            text,
            start: loc.start as u32,
            end: loc.end as u32,
        }
    }

    /// Returns the source text covered by this span.
    pub fn source_text(&self) -> &'cx str {
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
    fn ident() {
        // Proves `Ident` preserves the parsed identifier text.
        let ccx = syn_sem_common::CommonCx::new();
        let scx = SyntaxCx::new(&ccx);

        // Non-empty ident
        let ident = parse::<syn::Ident, Ident>(&scx, "A");
        assert_eq!(&*ident, "A");
    }
}
