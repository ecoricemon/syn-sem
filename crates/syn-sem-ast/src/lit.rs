use crate::{FromSyn, Span, SyntaxContext};
use any_intern::Interned;
use std::str::FromStr;
use syn_sem_macros::CheckDropless;

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Lit<'scx> {
    Int(LitInt<'scx>),
    Float(LitFloat<'scx>),
    Bool(LitBool<'scx>),
}

impl<'scx> Lit<'scx> {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Int(v) => v.base10_digits(),
            Self::Float(v) => v.base10_digits(),
            Self::Bool(v) => v.as_str(),
        }
    }
}

impl<'scx> FromSyn<'scx, syn::Lit> for Lit<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::Lit) -> Self {
        match input {
            syn::Lit::Int(v) => Self::Int(LitInt::from_syn(scx, v)),
            syn::Lit::Float(v) => Self::Float(LitFloat::from_syn(scx, v)),
            syn::Lit::Bool(v) => Self::Bool(LitBool::from_syn(scx, v)),
            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct LitInt<'scx> {
    pub literal: Interned<'scx, str>,
    pub span: Span<'scx>,
}

impl LitInt<'_> {
    pub fn base10_digits(&self) -> &str {
        &self.literal
    }

    pub fn base10_parse<F: FromStr>(&self) -> Result<F, F::Err> {
        self.base10_digits().parse()
    }
}

impl<'scx> FromSyn<'scx, syn::LitInt> for LitInt<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::LitInt) -> Self {
        Self {
            literal: scx.intern(input.base10_digits()),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct LitFloat<'scx> {
    pub literal: Interned<'scx, str>,
    pub span: Span<'scx>,
}

impl LitFloat<'_> {
    pub fn base10_digits(&self) -> &str {
        &self.literal
    }

    pub fn base10_parse<F: FromStr>(&self) -> Result<F, F::Err> {
        self.base10_digits().parse()
    }
}

impl<'scx> FromSyn<'scx, syn::LitFloat> for LitFloat<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::LitFloat) -> Self {
        Self {
            literal: scx.intern(input.base10_digits()),
            span: Span::from_locatable(scx, input),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct LitBool<'scx> {
    pub value: bool,
    pub span: Span<'scx>,
}

impl LitBool<'_> {
    pub fn as_str(&self) -> &'static str {
        match self.value {
            true => "true",
            false => "false",
        }
    }
}

impl<'scx> FromSyn<'scx, syn::LitBool> for LitBool<'scx> {
    fn from_syn(scx: &'scx SyntaxContext, input: &syn::LitBool) -> Self {
        Self {
            value: input.value,
            span: Span::from_locatable(scx, input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    #[test]
    fn test_lit_int() {
        let scx = create_context();
        let value = parse::<syn::LitInt, LitInt>(&scx, "1");
        assert_eq!(value.base10_parse::<i32>().unwrap(), 1);
    }

    #[test]
    fn test_lit_float() {
        let scx = create_context();
        let value = parse::<syn::LitFloat, LitFloat>(&scx, "1.");
        assert_eq!(value.base10_parse::<f32>().unwrap(), 1.);
    }

    #[test]
    fn test_lit_bool() {
        let scx = create_context();

        let value = parse::<syn::LitBool, LitBool>(&scx, "true");
        assert!(value.value);
        let value = parse::<syn::LitBool, LitBool>(&scx, "false");
        assert!(!value.value);
    }
}
