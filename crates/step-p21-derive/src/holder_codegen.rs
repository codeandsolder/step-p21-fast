//! Common code generation for named ENTITY structs and tuple TYPE declarations.

use proc_macro_error2::abort_call_site;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;

use crate::{
    common::{
        as_holder_ident, as_holder_visitor, as_visitor_ident, serde_crate,
        step_p21_crate,
    },
    field_type::FieldType,
    holder_attr::HolderAttr,
};

pub fn table_arg() -> syn::Ident {
    syn::Ident::new("table", Span::call_site())
}

/// Produce the same reference resolution contract for named and tuple holders.
pub fn impl_entity_table(
    ident: &syn::Ident,
    attr: &HolderAttr,
) -> TokenStream2 {
    let HolderAttr { table, field, .. } = attr;
    let holder_ident = as_holder_ident(ident);
    let step_p21 = step_p21_crate();

    quote! {
        #[automatically_derived]
        impl #step_p21::tables::EntityTable<#holder_ident> for #table {
            fn get_owned(&self, entity_id: u64) -> #step_p21::error::Result<#ident> {
                #step_p21::tables::get_owned(self, &self.#field, entity_id)
            }
            fn owned_iter<'table>(&'table self) -> Box<dyn Iterator<Item = #step_p21::error::Result<#ident>> + 'table> {
                #step_p21::tables::owned_iter(self, &self.#field)
            }
        }
    }
}

pub fn impl_with_visitor(ident: &syn::Ident) -> TokenStream2 {
    let step_p21 = step_p21_crate();
    let visitor_ident = as_holder_visitor(ident);
    let holder_ident = as_holder_ident(ident);

    quote! {
        #[automatically_derived]
        impl #step_p21::tables::WithVisitor for #holder_ident {
            type Visitor = #visitor_ident;
            fn visitor_new() -> Self::Visitor {
                #visitor_ident {}
            }
        }
    }
}

pub fn impl_deserialize(
    ident: &syn::Ident,
    name: &str,
    field_count: usize,
) -> TokenStream2 {
    let visitor_ident = as_visitor_ident(ident);
    let serde = serde_crate();
    quote! {
        #[automatically_derived]
        impl<'de> #serde::de::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
            where
                D: #serde::de::Deserializer<'de>,
            {
                deserializer.deserialize_tuple_struct(#name, #field_count, #visitor_ident {})
            }
        }
    }
}

/// Only the final constructor differs between named and tuple records. The
/// tuple/map visitor and all of its error reporting should stay identical.
pub fn def_visitor(
    ident: &syn::Ident,
    name: &str,
    attributes: &[syn::Ident],
    construct: &TokenStream2,
) -> TokenStream2 {
    let visitor_ident = as_visitor_ident(ident);
    let field_count = attributes.len();
    let indexes = 0..field_count;
    let serde = serde_crate();

    quote! {
        #[doc(hidden)]
        pub struct #visitor_ident;

        #[automatically_derived]
        impl<'de> #serde::de::Visitor<'de> for #visitor_ident {
            type Value = #ident;
            fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                write!(formatter, #name)
            }

            fn visit_seq<A>(self, mut seq: A) -> ::std::result::Result<Self::Value, A::Error>
            where
                A: #serde::de::SeqAccess<'de>,
            {
                use #serde::de::Error as _;
                if let Some(size) = seq.size_hint() {
                    if size != #field_count {
                        return Err(A::Error::invalid_length(size, &self));
                    }
                }
                #(
                    let #attributes = seq.next_element()?.ok_or_else(|| {
                        A::Error::invalid_length(#indexes, &self)
                    })?;
                )*
                Ok(#construct)
            }

            fn visit_map<A>(self, mut map: A) -> ::std::result::Result<Self::Value, A::Error>
            where
                A: #serde::de::MapAccess<'de>,
            {
                use #serde::de::{Error as _, Unexpected};
                let key: String = map.next_key()?.ok_or_else(|| {
                    A::Error::invalid_length(0, &self)
                })?;
                if key != #name {
                    return Err(A::Error::invalid_value(Unexpected::Other(&key), &self));
                }
                map.next_value()
            }
        }
    }
}

/// Normalize the shared placeholder conversion for all field shapes; only
/// the field access and final struct construction differ.
pub fn into_owned_value(
    field_type: &FieldType,
    value: &TokenStream2,
    table: &syn::Ident,
) -> TokenStream2 {
    match field_type {
        FieldType::Path(_) => quote! { #value.into_owned(#table)? },
        FieldType::Optional(_) => quote! {
            #value.map(|holder| holder.into_owned(#table)).transpose()?
        },
        FieldType::List(_) => quote! {
            #value.into_iter()
                .map(|holder| holder.into_owned(#table))
                .collect::<::std::result::Result<Vec<_>, _>>()?
        },
        FieldType::Boxed(_) => abort_call_site!("Unexpected Box<T>"),
    }
}
