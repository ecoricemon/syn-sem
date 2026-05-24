use proc_macro2::TokenStream as TokenStream2;
use std::mem;

// Allow dead code for future use
#[allow(dead_code)]
/// Convenience accessors for `syn` nodes that carry attributes.
pub trait AttributeHelper {
    /// Returns immutable attributes if this node has an attribute list.
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>>;

    /// Returns mutable attributes if this node has an attribute list.
    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>>;

    /// Finds an attribute with the given path.
    fn get_attribute(&self, path: &str) -> Option<&syn::Attribute> {
        self.get_attributes()?
            .iter()
            .find(|attr| attr.path().is_ident(path))
    }

    /// Finds a mutable attribute with the given path.
    fn get_mut_attribute(&mut self, path: &str) -> Option<&mut syn::Attribute> {
        self.get_mut_attributes()?
            .iter_mut()
            .find(|attr| attr.path().is_ident(path))
    }

    /// Returns whether this node has an attribute with the given path.
    fn contains_attribute(&self, path: &str) -> bool {
        let Some(attrs) = self.get_attributes() else {
            return false;
        };
        attrs.iter().any(|attr| attr.path().is_ident(path))
    }

    /// Removes all attributes with the given path.
    fn remove_attribute(&mut self, path: &str) {
        let Some(attrs) = self.get_mut_attributes() else {
            return;
        };
        attrs.retain(|attr| !attr.path().is_ident(path))
    }

    /// Replaces all attributes and returns the previous list.
    fn replace_attributes(&mut self, new: Vec<syn::Attribute>) -> Vec<syn::Attribute> {
        let Some(old) = self.get_mut_attributes() else {
            return Vec::new();
        };
        mem::replace(old, new)
    }

    /// Expands this vector by attaching the given value to the front of this vector.
    fn insert_front(&mut self, mut front: Vec<syn::Attribute>) {
        let Some(this) = self.get_mut_attributes() else {
            return;
        };
        front.append(this);
        let _ = mem::replace(this, front);
    }

    /// #\[path(inner)\]
    fn get_attribute_inner(&self, path: &str) -> Option<&TokenStream2> {
        let attr = self.get_attribute(path)?;
        if let syn::Meta::List(l) = &attr.meta {
            Some(&l.tokens)
        } else {
            None
        }
    }

    /// #\[path = value\]
    fn get_attribute_value(&self, path: &str) -> Option<&syn::Expr> {
        let attr = self.get_attribute(path)?;
        if let syn::Meta::NameValue(nv) = &attr.meta {
            Some(&nv.value)
        } else {
            None
        }
    }
}

macro_rules! impl_attribute_helper_for_simple {
    ($ty:ty) => {
        impl AttributeHelper for $ty {
            fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
                Some(&self.attrs)
            }

            fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
                Some(&mut self.attrs)
            }
        }
    };
}

impl_attribute_helper_for_simple!(syn::ExprArray);
impl_attribute_helper_for_simple!(syn::ExprAssign);
impl_attribute_helper_for_simple!(syn::ExprAsync);
impl_attribute_helper_for_simple!(syn::ExprAwait);
impl_attribute_helper_for_simple!(syn::ExprBinary);
impl_attribute_helper_for_simple!(syn::ExprBlock);
impl_attribute_helper_for_simple!(syn::ExprBreak);
impl_attribute_helper_for_simple!(syn::ExprCall);
impl_attribute_helper_for_simple!(syn::ExprCast);
impl_attribute_helper_for_simple!(syn::ExprClosure);
impl_attribute_helper_for_simple!(syn::ExprConst);
impl_attribute_helper_for_simple!(syn::ExprContinue);
impl_attribute_helper_for_simple!(syn::ExprField);
impl_attribute_helper_for_simple!(syn::ExprForLoop);
impl_attribute_helper_for_simple!(syn::ExprGroup);
impl_attribute_helper_for_simple!(syn::ExprIf);
impl_attribute_helper_for_simple!(syn::ExprIndex);
impl_attribute_helper_for_simple!(syn::ExprInfer);
impl_attribute_helper_for_simple!(syn::ExprLet);
impl_attribute_helper_for_simple!(syn::ExprLit);
impl_attribute_helper_for_simple!(syn::ExprLoop);
impl_attribute_helper_for_simple!(syn::ExprMacro);
impl_attribute_helper_for_simple!(syn::ExprMatch);
impl_attribute_helper_for_simple!(syn::ExprMethodCall);
impl_attribute_helper_for_simple!(syn::ExprParen);
impl_attribute_helper_for_simple!(syn::ExprPath);
impl_attribute_helper_for_simple!(syn::ExprRange);
impl_attribute_helper_for_simple!(syn::ExprRawAddr);
impl_attribute_helper_for_simple!(syn::ExprReference);
impl_attribute_helper_for_simple!(syn::ExprRepeat);
impl_attribute_helper_for_simple!(syn::ExprReturn);
impl_attribute_helper_for_simple!(syn::ExprStruct);
impl_attribute_helper_for_simple!(syn::ExprTry);
impl_attribute_helper_for_simple!(syn::ExprTryBlock);
impl_attribute_helper_for_simple!(syn::ExprTuple);
impl_attribute_helper_for_simple!(syn::ExprUnary);
impl_attribute_helper_for_simple!(syn::ExprUnsafe);
impl_attribute_helper_for_simple!(syn::ExprWhile);
impl_attribute_helper_for_simple!(syn::ExprYield);
impl_attribute_helper_for_simple!(syn::Field);
impl_attribute_helper_for_simple!(syn::ItemConst);
impl_attribute_helper_for_simple!(syn::ItemMod);
impl_attribute_helper_for_simple!(syn::ItemStruct);

impl AttributeHelper for syn::Item {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        match self {
            syn::Item::Const(v) => v.get_attributes(),
            syn::Item::Mod(v) => v.get_attributes(),
            syn::Item::Struct(v) => v.get_attributes(),
            _ => None,
        }
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        match self {
            syn::Item::Const(v) => v.get_mut_attributes(),
            syn::Item::Mod(v) => v.get_mut_attributes(),
            syn::Item::Struct(v) => v.get_mut_attributes(),
            _ => None,
        }
    }
}

impl AttributeHelper for syn::Expr {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        match self {
            Self::Array(v) => v.get_attributes(),
            Self::Assign(v) => v.get_attributes(),
            Self::Async(v) => v.get_attributes(),
            Self::Await(v) => v.get_attributes(),
            Self::Binary(v) => v.get_attributes(),
            Self::Block(v) => v.get_attributes(),
            Self::Break(v) => v.get_attributes(),
            Self::Call(v) => v.get_attributes(),
            Self::Cast(v) => v.get_attributes(),
            Self::Closure(v) => v.get_attributes(),
            Self::Const(v) => v.get_attributes(),
            Self::Continue(v) => v.get_attributes(),
            Self::Field(v) => v.get_attributes(),
            Self::ForLoop(v) => v.get_attributes(),
            Self::Group(v) => v.get_attributes(),
            Self::If(v) => v.get_attributes(),
            Self::Index(v) => v.get_attributes(),
            Self::Infer(v) => v.get_attributes(),
            Self::Let(v) => v.get_attributes(),
            Self::Lit(v) => v.get_attributes(),
            Self::Loop(v) => v.get_attributes(),
            Self::Macro(v) => v.get_attributes(),
            Self::Match(v) => v.get_attributes(),
            Self::MethodCall(v) => v.get_attributes(),
            Self::Paren(v) => v.get_attributes(),
            Self::Path(v) => v.get_attributes(),
            Self::Range(v) => v.get_attributes(),
            Self::RawAddr(v) => v.get_attributes(),
            Self::Reference(v) => v.get_attributes(),
            Self::Repeat(v) => v.get_attributes(),
            Self::Return(v) => v.get_attributes(),
            Self::Struct(v) => v.get_attributes(),
            Self::Try(v) => v.get_attributes(),
            Self::TryBlock(v) => v.get_attributes(),
            Self::Tuple(v) => v.get_attributes(),
            Self::Unary(v) => v.get_attributes(),
            Self::Unsafe(v) => v.get_attributes(),
            Self::Verbatim(_) => None,
            Self::While(v) => v.get_attributes(),
            Self::Yield(v) => v.get_attributes(),
            _ => unreachable!("non-exhaustive"),
        }
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        match self {
            Self::Array(v) => v.get_mut_attributes(),
            Self::Assign(v) => v.get_mut_attributes(),
            Self::Async(v) => v.get_mut_attributes(),
            Self::Await(v) => v.get_mut_attributes(),
            Self::Binary(v) => v.get_mut_attributes(),
            Self::Block(v) => v.get_mut_attributes(),
            Self::Break(v) => v.get_mut_attributes(),
            Self::Call(v) => v.get_mut_attributes(),
            Self::Cast(v) => v.get_mut_attributes(),
            Self::Closure(v) => v.get_mut_attributes(),
            Self::Const(v) => v.get_mut_attributes(),
            Self::Continue(v) => v.get_mut_attributes(),
            Self::Field(v) => v.get_mut_attributes(),
            Self::ForLoop(v) => v.get_mut_attributes(),
            Self::Group(v) => v.get_mut_attributes(),
            Self::If(v) => v.get_mut_attributes(),
            Self::Index(v) => v.get_mut_attributes(),
            Self::Infer(v) => v.get_mut_attributes(),
            Self::Let(v) => v.get_mut_attributes(),
            Self::Lit(v) => v.get_mut_attributes(),
            Self::Loop(v) => v.get_mut_attributes(),
            Self::Macro(v) => v.get_mut_attributes(),
            Self::Match(v) => v.get_mut_attributes(),
            Self::MethodCall(v) => v.get_mut_attributes(),
            Self::Paren(v) => v.get_mut_attributes(),
            Self::Path(v) => v.get_mut_attributes(),
            Self::Range(v) => v.get_mut_attributes(),
            Self::RawAddr(v) => v.get_mut_attributes(),
            Self::Reference(v) => v.get_mut_attributes(),
            Self::Repeat(v) => v.get_mut_attributes(),
            Self::Return(v) => v.get_mut_attributes(),
            Self::Struct(v) => v.get_mut_attributes(),
            Self::Try(v) => v.get_mut_attributes(),
            Self::TryBlock(v) => v.get_mut_attributes(),
            Self::Tuple(v) => v.get_mut_attributes(),
            Self::Unary(v) => v.get_mut_attributes(),
            Self::Unsafe(v) => v.get_mut_attributes(),
            Self::Verbatim(_) => None,
            Self::While(v) => v.get_mut_attributes(),
            Self::Yield(v) => v.get_mut_attributes(),
            _ => unreachable!("non-exhaustive"),
        }
    }
}
