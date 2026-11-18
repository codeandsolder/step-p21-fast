use super::{as_holder_ident, step_p21_crate};

use proc_macro_error2::{Diagnostic, Level, OptionExt};
use std::convert::{TryFrom, TryInto};

/// step_p11-generated field type
#[derive(Debug, Clone)]
pub enum FieldType {
    /// Like `T`
    Path(syn::Path),
    /// Like `Option<T>`
    Optional(Box<Self>),
    /// Like `Vec<T>`
    List(Box<Self>),
    /// Like `Box<T>`
    Boxed(Box<Self>),
}

impl FieldType {
    pub fn into_holder(self) -> Self {
        match self {
            Self::Path(path) => {
                let syn::Path {
                    leading_colon,
                    mut segments,
                } = path;
                let last_seg = segments
                    .last_mut()
                    .expect_or_abort("field path has no segments");
                match &mut last_seg.arguments {
                    syn::PathArguments::None => {
                        last_seg.ident = as_holder_ident(&last_seg.ident);
                    }
                    _ => unreachable!(),
                }
                Self::Path(syn::Path {
                    leading_colon,
                    segments,
                })
            }
            Self::Optional(ty) => {
                let holder = ty.into_holder();
                Self::Optional(Box::new(holder))
            }
            Self::List(ty) => {
                let holder = ty.into_holder();
                Self::List(Box::new(holder))
            }
            Self::Boxed(ty) => {
                let holder = ty.into_holder();
                Self::Boxed(Box::new(holder))
            }
        }
    }

    pub fn into_place_holder(self) -> Self {
        let step_p21 = step_p21_crate();
        match self {
            Self::Path(path) => {
                let path =
                    syn::parse_quote! { #step_p21::tables::PlaceHolder<#path> };
                Self::Path(path)
            }
            Self::Optional(ty) => {
                let place_holder = ty.into_place_holder();
                Self::Optional(Box::new(place_holder))
            }
            Self::List(ty) => {
                let place_holder = ty.into_place_holder();
                Self::List(Box::new(place_holder))
            }
            Self::Boxed(ty) => {
                let place_holder = ty.into_place_holder();
                Self::Boxed(Box::new(place_holder))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnsupportedTypeError {}

impl From<UnsupportedTypeError> for Diagnostic {
    fn from(_: UnsupportedTypeError) -> Self {
        Self::new(
            Level::Error,
            "Unsupported Type for step_p21 and step_p11".to_string(),
        )
    }
}

impl From<FieldType> for syn::Type {
    fn from(field_type: FieldType) -> Self {
        let path = match field_type {
            FieldType::Path(path) => path,
            FieldType::Optional(ty) => {
                let ty: Self = (*ty).into();
                syn::parse_quote! { Option<#ty> }
            }
            FieldType::List(ty) => {
                let ty: Self = (*ty).into();
                syn::parse_quote! { Vec<#ty> }
            }
            FieldType::Boxed(ty) => {
                let ty: Self = (*ty).into();
                syn::parse_quote! { Box<#ty> }
            }
        };
        // syn 3 added `attrs` to `TypePath`; a synthesised type carries none.
        Self::Path(syn::TypePath {
            attrs: Vec::new(),
            qself: None,
            path,
        })
    }
}

impl TryFrom<syn::Type> for FieldType {
    type Error = UnsupportedTypeError;

    fn try_from(ty: syn::Type) -> Result<Self, Self::Error> {
        let syn::Type::Path(syn::TypePath { path, .. }) = ty else {
            return Err(UnsupportedTypeError {});
        };

        let syn::Path { segments, .. } = &path;
        let last_seg = segments.last().ok_or(UnsupportedTypeError {})?;

        match &last_seg.arguments {
            syn::PathArguments::None => Ok(Self::Path(path.clone())),
            syn::PathArguments::AngleBracketed(
                syn::AngleBracketedGenericArguments { args, .. },
            ) => {
                if args.len() != 1 {
                    return Err(UnsupportedTypeError {});
                }
                if let syn::GenericArgument::Type(ty) = &args[0] {
                    let ty = Box::new(ty.clone().try_into()?);
                    if last_seg.ident == "Option" {
                        return Ok(Self::Optional(ty));
                    }
                    if last_seg.ident == "Vec" {
                        return Ok(Self::List(ty));
                    }
                    if last_seg.ident == "Box" {
                        return Ok(Self::Boxed(ty));
                    }
                }
                Err(UnsupportedTypeError {})
            }
            syn::PathArguments::Parenthesized(_) => {
                Err(UnsupportedTypeError {})
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_type_convert() {
        let ty: syn::Type = syn::parse_str("T").unwrap();
        let f = ty.clone().try_into().unwrap();
        assert!(matches!(f, FieldType::Path(_)));
        assert_eq!(<FieldType as Into<syn::Type>>::into(f), ty);

        let ty: syn::Type = syn::parse_str("Option<T>").unwrap();
        let f = ty.clone().try_into().unwrap();
        assert!(matches!(f, FieldType::Optional(_)));
        assert_eq!(<FieldType as Into<syn::Type>>::into(f), ty);

        let ty: syn::Type = syn::parse_str("Vec<T>").unwrap();
        let f = ty.clone().try_into().unwrap();
        assert!(matches!(f, FieldType::List(_)));
        assert_eq!(<FieldType as Into<syn::Type>>::into(f), ty);

        let ty: syn::Type = syn::parse_str("Option<Vec<T>>").unwrap();
        let f = ty.clone().try_into().unwrap();
        if let FieldType::Optional(ty) = &f {
            assert!(matches!(**ty, FieldType::List(_)));
        } else {
            panic!()
        }
        assert_eq!(<FieldType as Into<syn::Type>>::into(f), ty);
    }

    #[test]
    fn as_holder() {
        let ty: syn::Type = syn::parse_str("T").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let holder = f.into_holder();
        let ans: syn::Type = syn::parse_str("THolder").unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(holder), ans);

        let ty: syn::Type = syn::parse_str("Option<T>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let holder = f.into_holder();
        let ans: syn::Type = syn::parse_str("Option<THolder>").unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(holder), ans);

        let ty: syn::Type = syn::parse_str("Vec<T>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let holder = f.into_holder();
        let ans: syn::Type = syn::parse_str("Vec<THolder>").unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(holder), ans);

        let ty: syn::Type = syn::parse_str("Option<Vec<T>>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let holder = f.into_holder();
        let ans: syn::Type = syn::parse_str("Option<Vec<THolder>>").unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(holder), ans);
    }

    #[test]
    fn as_place_holder() {
        let ty: syn::Type = syn::parse_str("T").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let place_holder = f.into_holder().into_place_holder();
        let ans: syn::Type =
            syn::parse_str("::step_p21::tables::PlaceHolder<THolder>").unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(place_holder), ans);

        let ty: syn::Type = syn::parse_str("Option<T>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let place_holder = f.into_holder().into_place_holder();
        let ans: syn::Type =
            syn::parse_str("Option<::step_p21::tables::PlaceHolder<THolder>>")
                .unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(place_holder), ans);

        let ty: syn::Type = syn::parse_str("Vec<T>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let place_holder = f.into_holder().into_place_holder();
        let ans: syn::Type =
            syn::parse_str("Vec<::step_p21::tables::PlaceHolder<THolder>>")
                .unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(place_holder), ans);

        let ty: syn::Type = syn::parse_str("Option<Vec<T>>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let place_holder = f.into_holder().into_place_holder();
        let ans: syn::Type = syn::parse_str(
            "Option<Vec<::step_p21::tables::PlaceHolder<THolder>>>",
        )
        .unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(place_holder), ans);
    }
}
