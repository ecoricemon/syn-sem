use crate::{FromSyn, InputDesc, Span, SyntaxCx};
use any_intern::Interned;
use std::str::FromStr;
use syn_sem_macros::CheckDropless;

/// A literal expression value supported by the semantic AST.
///
/// Examples include `1`, `1.0`, and `true`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Lit<'cx> {
    /// Integer literal.
    Int(LitInt<'cx>),
    /// Floating-point literal.
    Float(LitFloat<'cx>),
    /// Boolean literal.
    Bool(LitBool<'cx>),
}

impl<'cx> Lit<'cx> {
    /// Returns the normalized literal text.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Int(v) => v.base10_digits(),
            Self::Float(v) => v.base10_digits(),
            Self::Bool(v) => v.as_str(),
        }
    }

    /// Returns the source span of the literal.
    pub fn span(&self) -> Span<'cx> {
        match self {
            Self::Int(v) => v.span,
            Self::Float(v) => v.span,
            Self::Bool(v) => v.span,
        }
    }
}

impl<'cx> FromSyn<'cx, syn::Lit> for Lit<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Lit>) -> Self {
        match desc.input {
            syn::Lit::Int(v) => Self::Int(LitInt::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Lit::Float(v) => Self::Float(LitFloat::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Lit::Bool(v) => Self::Bool(LitBool::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            _ => todo!(),
        }
    }
}

/// An integer literal.
///
/// For example, `42` or `0xff` is stored by its base-10 digits for semantic parsing.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct LitInt<'cx> {
    /// Normalized base-10 digits.
    pub literal: Interned<'cx, str>,
    /// Source span of the literal.
    pub span: Span<'cx>,
}

impl LitInt<'_> {
    /// Returns the normalized base-10 digits.
    pub fn base10_digits(&self) -> &str {
        &self.literal
    }

    /// Parses the normalized base-10 digits.
    pub fn base10_parse<F: FromStr>(&self) -> Result<F, F::Err> {
        self.base10_digits().parse()
    }
}

impl<'cx> FromSyn<'cx, syn::LitInt> for LitInt<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::LitInt>) -> Self {
        Self {
            literal: scx.intern(desc.input.base10_digits()),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A floating-point literal.
///
/// For example, `1.0` or `3.14` is stored by its base-10 digits for semantic parsing.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct LitFloat<'cx> {
    /// Normalized base-10 digits.
    pub literal: Interned<'cx, str>,
    /// Source span of the literal.
    pub span: Span<'cx>,
}

impl LitFloat<'_> {
    /// Returns the normalized base-10 digits.
    pub fn base10_digits(&self) -> &str {
        &self.literal
    }

    /// Parses the normalized base-10 digits.
    pub fn base10_parse<F: FromStr>(&self) -> Result<F, F::Err> {
        self.base10_digits().parse()
    }
}

impl<'cx> FromSyn<'cx, syn::LitFloat> for LitFloat<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::LitFloat>) -> Self {
        Self {
            literal: scx.intern(desc.input.base10_digits()),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A boolean literal.
///
/// Examples are `true` and `false`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct LitBool<'cx> {
    /// Boolean value.
    pub value: bool,
    /// Source span of the literal.
    pub span: Span<'cx>,
}

impl LitBool<'_> {
    /// Returns `true` or `false`.
    pub fn as_str(&self) -> &'static str {
        match self.value {
            true => "true",
            false => "false",
        }
    }
}

impl<'cx> FromSyn<'cx, syn::LitBool> for LitBool<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::LitBool>) -> Self {
        Self {
            value: desc.input.value,
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn lit_int() {
        // Proves integer literals preserve their parsed numeric value.
        let ccx = syn_sem_common::CommonCx::new();
        let cx = SyntaxCx::new(&ccx);
        let value = parse::<syn::LitInt, LitInt>(&cx, "1");
        assert_eq!(value.base10_parse::<i32>().unwrap(), 1);
    }

    #[test]
    fn lit_float() {
        // Proves float literals preserve their parsed numeric value.
        let ccx = syn_sem_common::CommonCx::new();
        let cx = SyntaxCx::new(&ccx);
        let value = parse::<syn::LitFloat, LitFloat>(&cx, "1.");
        assert_eq!(value.base10_parse::<f32>().unwrap(), 1.);
    }

    #[test]
    fn lit_bool() {
        // Proves boolean literals preserve true and false values.
        let ccx = syn_sem_common::CommonCx::new();
        let cx = SyntaxCx::new(&ccx);

        let value = parse::<syn::LitBool, LitBool>(&cx, "true");
        assert!(value.value);
        let value = parse::<syn::LitBool, LitBool>(&cx, "false");
        assert!(!value.value);
    }
}
