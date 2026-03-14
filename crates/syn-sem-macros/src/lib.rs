use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

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
