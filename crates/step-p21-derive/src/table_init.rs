use inflector::Inflector;
use proc_macro_error2::{OptionExt, abort, abort_call_site};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::common::step_p21_crate;

pub fn derive_table_init(ast: &syn::DeriveInput) -> TokenStream2 {
    let ident = &ast.ident;
    let syn::Data::Struct(st) = &ast.data else {
        abort_call_site!("TableInit only supports structs")
    };

    let dispatch = match &st.fields {
        syn::Fields::Named(fields) => {
            let arms = fields.named.iter().map(|field| {
                let member = field.ident.as_ref().expect_or_abort("named table field");
                let name = member.to_string().to_screaming_snake_case();
                quote! { #name => { insert_record(&mut self.#member, *id, record)?; true }, }
            });
            quote! {
                match record.name.as_str() {
                    #(#arms)*
                    _ => false,
                }
            }
        }
        syn::Fields::Unnamed(fields) => {
            let step_p21 = step_p21_crate();
            let entries = fields.unnamed.iter().enumerate().map(|(index, field)| {
                let member = syn::Index::from(index);
                let holder = holder_type(&field.ty);
                quote! {
                    if !matched && record.name == <#holder as #step_p21::tables::Holder>::name() {
                        insert_record(&mut self.#member, *id, record)?;
                        matched = true;
                    }
                }
            });
            // Associated Holder::name() gives the EXPRESS entity name
            // regardless of how the table field type was spelled.
            quote! {
                let mut matched = false;
                #(#entries)*
                matched
            }
        }
        syn::Fields::Unit => abort_call_site!("Unit struct is not supported."),
    };

    // A Part 21 entity instance ID is unique across the entire exchange
    // structure, not just within a map for one EXPRESS entity type. Recover
    // existing IDs when appending a later DATA section, too.
    let members = table_members(&st.fields);

    let step_p21 = step_p21_crate();
    quote! {
        #[automatically_derived]
        impl #step_p21::tables::TableInit for #ident {
            fn append_data_section(
                &mut self,
                data_sec: &#step_p21::ast::DataSection
            ) -> #step_p21::error::Result<()> {
                use #step_p21::{
                    error::Error,
                    tables::insert_record,
                    ast::EntityInstance,
                };
                // Keep file-wide entity identity independent of the storage
                // map selected by each record's EXPRESS type.
                let mut seen_ids = ::std::collections::HashSet::with_capacity(
                    data_sec.entities.len() #(+ self.#members.len())*
                );
                #(
                    for &id in self.#members.keys() {
                        if !seen_ids.insert(id) {
                            return Err(Error::DuplicatedEntity(id));
                        }
                    }
                )*
                for entity in &data_sec.entities {
                    match entity {
                        EntityInstance::Simple { id, record } => {
                            if !seen_ids.insert(*id) {
                                return Err(Error::DuplicatedEntity(*id));
                            }
                            let recognized = { #dispatch };
                            if !recognized {
                                return Err(Error::UnknownEntityName {
                                    entity_name: record.name.clone(),
                                    schema: String::new(),
                                });
                            }
                        }
                        EntityInstance::Complex { .. } => {
                            return Err(Error::DeserializeFailed(
                                "TableInit does not support complex entity instances".to_owned()
                            ));
                        }
                    }
                }
                Ok(())
            }
        }

        #[automatically_derived]
        impl ::std::str::FromStr for #ident {
            type Err = #step_p21::error::Error;
            fn from_str(input: &str) -> #step_p21::error::Result<Self> {
                use #step_p21::{tables::TableInit, ast::DataSection};
                let data_sec = DataSection::from_str(input)?;
                Self::from_data_section(&data_sec)
            }
        }
    }
}

/// Enumerate the storage maps independently from their dispatch rules.
fn table_members(fields: &syn::Fields) -> Vec<syn::Member> {
    match fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(|field| {
                syn::Member::Named(
                    field.ident.clone().expect_or_abort("named table field"),
                )
            })
            .collect(),
        syn::Fields::Unnamed(fields) => (0..fields.unnamed.len())
            .map(|index| syn::Member::Unnamed(syn::Index::from(index)))
            .collect(),
        syn::Fields::Unit => abort_call_site!("Unit struct is not supported."),
    }
}

/// A positional table field has a concrete `HashMap<u64, Holder, S>`
/// type; retain arbitrary hasher support while extracting its Holder type.
/// Avoid reconstructing entity names from Rust type spelling.
fn holder_type(ty: &syn::Type) -> &syn::Type {
    let syn::Type::Path(path) = ty else {
        abort!(ty, "tuple TableInit fields must be HashMap<u64, Holder>")
    };
    let segment = path.path.segments.last().expect_or_abort("empty type path");
    if segment.ident != "HashMap" {
        abort!(
            segment.ident,
            "tuple TableInit fields must be HashMap<u64, Holder>"
        )
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        abort!(segment, "HashMap must have a key and holder value type")
    };
    let Some(syn::GenericArgument::Type(value)) = args.args.iter().nth(1)
    else {
        abort!(segment, "HashMap must have a holder value type")
    };
    value
}
