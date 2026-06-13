//! Procedural macros used by extracted `syn-sem` crates.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    braced, parse::Parse, parse::ParseStream, parse_macro_input, Attribute, DeriveInput, Ident,
    Result, Visibility,
};

/// Causes panic if dropping the type has side effects.
///
/// But note that this may be false-positive. No panics means dropping the type has no side effects,
/// but this may cause panic on types that actually do not have side effects.
#[proc_macro_derive(CheckDropless)]
pub fn derive_check_dropless(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let ident = &ast.ident;

    quote! {
        const _: () = assert!(!core::mem::needs_drop::<#ident>());
    }
    .into()
}

/// Defines stable arena id newtypes backed by `usize`.
#[proc_macro]
pub fn define_id(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as IdDefinitions);
    let ids = input.ids.iter().map(|id| {
        let attrs = &id.attrs;
        let ident = &id.ident;
        let method_vis = id
            .method_vis
            .as_ref()
            .map_or_else(|| quote! { pub }, |method_vis| quote! { #method_vis });

        quote! {
            #(#attrs)*
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct #ident(pub(crate) usize);

            impl #ident {
                /// Creates an id from a raw index.
                ///
                /// This is intended for tests and serialization boundaries. Normal code should
                /// obtain ids from the owning arena.
                #method_vis const fn new(index: usize) -> Self {
                    Self(index)
                }

                /// Returns the raw index represented by this id.
                #method_vis const fn index(self) -> usize {
                    self.0
                }
            }

            impl std::fmt::Debug for #ident {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}({})", stringify!(#ident), self.0)
                }
            }

            impl std::fmt::Display for #ident {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.0.fmt(f)
                }
            }
        }
    });

    quote! {
        #(#ids)*
    }
    .into()
}

struct IdDefinitions {
    ids: Vec<IdDefinition>,
}

struct IdDefinition {
    attrs: Vec<Attribute>,
    method_vis: Option<Visibility>,
    ident: Ident,
}

impl Parse for IdDefinitions {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut ids = Vec::new();

        while !input.is_empty() {
            let content;
            braced!(content in input);
            let attrs = content.call(Attribute::parse_outer)?;
            let method_vis = if content.peek(syn::Token![pub]) {
                Some(content.parse()?)
            } else {
                None
            };
            let ident = content.parse()?;
            ids.push(IdDefinition {
                attrs,
                method_vis,
                ident,
            });
        }

        Ok(Self { ids })
    }
}
