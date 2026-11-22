use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro_error2::{ResultExt, abort_call_site};
use proc_macro2::Span;
use quote::format_ident;

use super::field_type::FieldType;

pub fn as_holder_visitor(input: &syn::Ident) -> syn::Ident {
    format_ident!("{}HolderVisitor", input)
}

pub fn as_holder_ident(input: &syn::Ident) -> syn::Ident {
    format_ident!("{}Holder", input)
}

pub fn as_holder_path(input: &syn::Type) -> syn::Type {
    let ft: FieldType = input
        .clone()
        .try_into()
        .expect_or_abort("as_holder! only accepts step_p11-generated type");
    ft.into_holder().into()
}

pub fn as_visitor_ident(input: &syn::Ident) -> syn::Ident {
    format_ident!("{}Visitor", input)
}

pub fn serde_crate() -> syn::Path {
    let step_p21 = step_p21_crate();
    syn::parse_quote!( #step_p21::serde )
}

pub fn itertools_crate() -> syn::Path {
    let step_p21 = step_p21_crate();
    syn::parse_quote!( #step_p21::itertools )
}

/// Resolve the dependency name, including aliases, without guessing the Cargo
/// target.
pub fn step_p21_crate() -> syn::Path {
    match crate_name("step-p21").unwrap_or_else(|error| {
        abort_call_site!("failed to locate step-p21 crate: {}", error)
    }) {
        // Within the package, the library aliases itself as step_p21.
        // Its other targets already import that library under this name.
        FoundCrate::Itself => syn::parse_quote!(::step_p21),
        FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, Span::call_site());
            syn::parse_quote!(::#ident)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holder_path() {
        let path = syn::parse_str("::some::Struct").unwrap();
        let holder = as_holder_path(&path);
        let ans = syn::parse_str("::some::StructHolder").unwrap();
        assert_eq!(holder, ans);
    }

    #[test]
    fn optional_holder_path() {
        let path = syn::parse_str("Option<::some::Struct>").unwrap();
        let holder = as_holder_path(&path);
        let ans = syn::parse_str("Option<::some::StructHolder>").unwrap();
        assert_eq!(holder, ans);
    }
}
