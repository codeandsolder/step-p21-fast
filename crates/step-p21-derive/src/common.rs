use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::format_ident;

use super::field_type::FieldType;

pub fn as_holder_visitor(input: &syn::Ident) -> syn::Ident {
    format_ident!("{}HolderVisitor", input)
}

pub fn as_holder_ident(input: &syn::Ident) -> syn::Ident {
    format_ident!("{}Holder", input)
}

pub fn as_holder_path(input: &syn::Type) -> syn::Result<syn::Type> {
    let ft = FieldType::try_from(input.clone())?;
    Ok(ft.into_holder()?.into())
}

pub fn as_visitor_ident(input: &syn::Ident) -> syn::Ident {
    format_ident!("{}Visitor", input)
}

/// Paths resolved once per macro invocation, including renamed dependencies.
pub struct CratePaths {
    pub step: syn::Path,
    pub serde: syn::Path,
    pub itertools: syn::Path,
}

impl CratePaths {
    pub fn resolve() -> syn::Result<Self> {
        let step: syn::Path = match crate_name("step-p21").map_err(|error| {
            syn::Error::new(
                Span::call_site(),
                format!("failed to locate step-p21 crate: {error}"),
            )
        })? {
            // The library aliases itself as step_p21; its other targets import
            // it.
            FoundCrate::Itself => syn::parse_quote!(::step_p21),
            FoundCrate::Name(name) => {
                let ident = syn::Ident::new(&name, Span::call_site());
                syn::parse_quote!(::#ident)
            }
        };
        let serde = syn::parse_quote!(#step::serde);
        let itertools = syn::parse_quote!(#step::itertools);
        Ok(Self {
            step,
            serde,
            itertools,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holder_path() {
        let path = syn::parse_str("::some::Struct").unwrap();
        let holder = as_holder_path(&path).unwrap();
        let ans = syn::parse_str("::some::StructHolder").unwrap();
        assert_eq!(holder, ans);
    }

    #[test]
    fn optional_holder_path() {
        let path = syn::parse_str("Option<::some::Struct>").unwrap();
        let holder = as_holder_path(&path).unwrap();
        let ans = syn::parse_str("Option<::some::StructHolder>").unwrap();
        assert_eq!(holder, ans);
    }
}
