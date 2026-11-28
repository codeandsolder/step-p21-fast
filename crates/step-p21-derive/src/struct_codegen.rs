//! Shared code generation for named ENTITY and positional TYPE records.

use inflector::Inflector;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::{
    common::{CratePaths, as_holder_ident},
    field_type::FieldType,
    holder_attr::HolderAttr,
    holder_codegen,
};

/// The syntax of the source record determines both its visitor constructor
/// and its generated holder fields. Lower each field once.
enum RecordShape<'a> {
    Named(&'a syn::FieldsNamed),
    Positional(&'a syn::FieldsUnnamed),
}

impl<'a> RecordShape<'a> {
    fn from_struct(
        ident: &syn::Ident,
        st: &'a syn::DataStruct,
    ) -> syn::Result<Self> {
        match &st.fields {
            syn::Fields::Named(fields) => Ok(Self::Named(fields)),
            syn::Fields::Unnamed(fields) => Ok(Self::Positional(fields)),
            syn::Fields::Unit => Err(syn::Error::new(
                ident.span(),
                "Unit struct is not supported.",
            )),
        }
    }

    const fn fields(
        &self,
    ) -> &syn::punctuated::Punctuated<syn::Field, syn::Token![,]> {
        match self {
            Self::Named(fields) => &fields.named,
            Self::Positional(fields) => &fields.unnamed,
        }
    }

    fn bindings(&self) -> syn::Result<Vec<syn::Ident>> {
        match self {
            Self::Named(fields) => fields
                .named
                .iter()
                .map(|field| {
                    field.ident.clone().ok_or_else(|| {
                        syn::Error::new_spanned(
                            field,
                            "expected named entity field",
                        )
                    })
                })
                .collect(),
            Self::Positional(fields) => Ok((0..fields.unnamed.len())
                .map(|i| format_ident!("a_{}", i))
                .collect()),
        }
    }

    fn visitor(
        &self,
        ident: &syn::Ident,
        name: &str,
        bindings: &[syn::Ident],
        paths: &CratePaths,
    ) -> TokenStream2 {
        let construct = match self {
            Self::Named(_) => quote! { #ident { #(#bindings),* } },
            Self::Positional(_) => quote! { #ident (#(#bindings),*) },
        };
        holder_codegen::def_visitor(ident, name, bindings, &construct, paths)
    }
}

struct LoweredField {
    ty: syn::Type,
    owned: TokenStream2,
}

fn lower_field(
    field: &syn::Field,
    value: &TokenStream2,
    table_arg: &syn::Ident,
    paths: &CratePaths,
) -> syn::Result<LoweredField> {
    let attributes = HolderAttr::parse(&field.attrs)?;
    if attributes.place_holder {
        let field_type = FieldType::try_from(field.ty.clone())?;
        let expression =
            holder_codegen::into_owned_value(&field_type, value, table_arg);
        let ty = field_type
            .into_holder()?
            .into_place_holder(&paths.step)
            .into();
        let owned = match &field.ident {
            Some(ident) => quote! { #ident: #expression },
            None => expression,
        };
        Ok(LoweredField { ty, owned })
    } else {
        Ok(LoweredField {
            ty: field.ty.clone(),
            owned: value.clone(),
        })
    }
}

pub fn derive_deserialize(
    ident: &syn::Ident,
    st: &syn::DataStruct,
    paths: &CratePaths,
) -> syn::Result<TokenStream2> {
    let shape = RecordShape::from_struct(ident, st)?;
    let bindings = shape.bindings()?;
    let name = ident.to_string().to_screaming_snake_case();
    let def_visitor_tt = shape.visitor(ident, &name, &bindings, paths);
    let impl_deserialize_tt =
        holder_codegen::impl_deserialize(ident, &name, bindings.len(), paths);
    Ok(quote! {
        #def_visitor_tt
        #impl_deserialize_tt
    })
}

pub fn derive_holder(
    ident: &syn::Ident,
    st: &syn::DataStruct,
    attr: &HolderAttr,
    paths: &CratePaths,
) -> syn::Result<TokenStream2> {
    let shape = RecordShape::from_struct(ident, st)?;
    let bindings = shape.bindings()?;
    let table = attr.table.as_ref().ok_or_else(|| {
        syn::Error::new(ident.span(), "missing holder table attribute")
    })?;
    let table_arg = holder_codegen::table_arg();
    let fields = shape
        .fields()
        .iter()
        .zip(&bindings)
        .enumerate()
        .map(|(i, (field, binding))| {
            let value = match shape {
                RecordShape::Named(_) => quote!(#binding),
                RecordShape::Positional(_) => {
                    let index = syn::Index::from(i);
                    quote!(self.#index)
                }
            };
            lower_field(field, &value, &table_arg, paths)
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let holder_ident = as_holder_ident(ident);
    let name = ident.to_string().to_screaming_snake_case();
    let holder_derives =
        holder_codegen::holder_derive_attrs(attr.derive_eq, false);
    let holder_types = fields.iter().map(|field| &field.ty);
    let into_owned = fields.iter().map(|field| &field.owned);
    let (definition, destructure, constructor) = match &shape {
        RecordShape::Named(_) => (
            quote! { { #(pub #bindings: #holder_types),* } },
            quote! { let #holder_ident { #(#bindings),* } = self; },
            quote! { #ident { #(#into_owned),* } },
        ),
        RecordShape::Positional(_) => (
            quote! { (#(pub #holder_types),*); },
            TokenStream2::new(),
            quote! { #ident (#(#into_owned),*) },
        ),
    };
    let def_holder_tt = quote! {
        /// Auto-generated by `#[derive(Holder)]`
        #holder_derives
        pub struct #holder_ident #definition
    };
    let step_p21 = &paths.step;
    let field_count = fields.len();
    let impl_holder_tt = quote! {
        #[automatically_derived]
        impl #step_p21::tables::IntoOwned for #holder_ident {
            type Table = #table;
            type Owned = #ident;
            fn into_owned(self, #table_arg: &Self::Table) -> #step_p21::error::Result<Self::Owned> {
                #destructure
                Ok(#constructor)
            }
        }
        #[automatically_derived]
        impl #step_p21::tables::Holder for #holder_ident {
            fn name() -> &'static str {
                #name
            }
            fn attr_len() -> usize {
                #field_count
            }
        }
    };
    let impl_entity_table_tt =
        holder_codegen::impl_entity_table(ident, attr, paths)?;
    let deserialize = if attr.generate_deserialize {
        let def_visitor_tt =
            shape.visitor(&holder_ident, &name, &bindings, paths);
        let impl_deserialize_tt = holder_codegen::impl_deserialize(
            &holder_ident,
            &name,
            field_count,
            paths,
        );
        let impl_with_visitor_tt =
            holder_codegen::impl_with_visitor(ident, paths);
        quote! {
            #def_visitor_tt
            #impl_deserialize_tt
            #impl_with_visitor_tt
        }
    } else {
        TokenStream2::new()
    };
    Ok(quote! {
        #def_holder_tt
        #impl_holder_tt
        #impl_entity_table_tt
        #deserialize
    })
}
