use super::as_holder_ident;

use std::convert::{TryFrom, TryInto};
use syn::spanned::Spanned;

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
    pub fn into_holder(self) -> syn::Result<Self> {
        Ok(match self {
            Self::Path(mut path) => {
                let span = path.span();
                let last = path.segments.last_mut().ok_or_else(|| {
                    syn::Error::new(span, "field path has no segments")
                })?;
                if !matches!(last.arguments, syn::PathArguments::None) {
                    return Err(syn::Error::new_spanned(
                        last,
                        "unsupported generic holder type",
                    ));
                }
                last.ident = as_holder_ident(&last.ident);
                Self::Path(path)
            }
            Self::Optional(ty) => Self::Optional(Box::new(ty.into_holder()?)),
            Self::List(ty) => Self::List(Box::new(ty.into_holder()?)),
            Self::Boxed(ty) => Self::Boxed(Box::new(ty.into_holder()?)),
        })
    }

    pub fn into_place_holder(self, step_p21: &syn::Path) -> Self {
        match self {
            Self::Path(path) => {
                let path =
                    syn::parse_quote! { #step_p21::tables::PlaceHolder<#path> };
                Self::Path(path)
            }
            Self::Optional(ty) => {
                let place_holder = ty.into_place_holder(step_p21);
                Self::Optional(Box::new(place_holder))
            }
            Self::List(ty) => {
                let place_holder = ty.into_place_holder(step_p21);
                Self::List(Box::new(place_holder))
            }
            Self::Boxed(ty) => {
                let place_holder = ty.into_place_holder(step_p21);
                Self::Boxed(Box::new(place_holder))
            }
        }
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
    type Error = syn::Error;

    fn try_from(ty: syn::Type) -> Result<Self, Self::Error> {
        let span = ty.span();
        let unsupported = || {
            syn::Error::new(span, "Unsupported Type for step_p21 and step_p11")
        };
        let syn::Type::Path(syn::TypePath {
            path, qself: None, ..
        }) = ty
        else {
            return Err(unsupported());
        };

        let syn::Path { segments, .. } = &path;
        let last_seg = segments.last().ok_or_else(unsupported)?;

        match &last_seg.arguments {
            syn::PathArguments::None => Ok(Self::Path(path.clone())),
            syn::PathArguments::AngleBracketed(
                syn::AngleBracketedGenericArguments { args, .. },
            ) => {
                if args.len() != 1 {
                    return Err(unsupported());
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
                Err(unsupported())
            }
            syn::PathArguments::Parenthesized(_) => Err(unsupported()),
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
        let holder = f.into_holder().unwrap();
        let ans: syn::Type = syn::parse_str("THolder").unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(holder), ans);

        let ty: syn::Type = syn::parse_str("Option<T>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let holder = f.into_holder().unwrap();
        let ans: syn::Type = syn::parse_str("Option<THolder>").unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(holder), ans);

        let ty: syn::Type = syn::parse_str("Vec<T>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let holder = f.into_holder().unwrap();
        let ans: syn::Type = syn::parse_str("Vec<THolder>").unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(holder), ans);

        let ty: syn::Type = syn::parse_str("Option<Vec<T>>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let holder = f.into_holder().unwrap();
        let ans: syn::Type = syn::parse_str("Option<Vec<THolder>>").unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(holder), ans);
    }

    #[test]
    fn as_place_holder() {
        let ty: syn::Type = syn::parse_str("T").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let place_holder = f
            .into_holder()
            .unwrap()
            .into_place_holder(&syn::parse_quote!(::step_p21));
        let ans: syn::Type =
            syn::parse_str("::step_p21::tables::PlaceHolder<THolder>").unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(place_holder), ans);

        let ty: syn::Type = syn::parse_str("Option<T>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let place_holder = f
            .into_holder()
            .unwrap()
            .into_place_holder(&syn::parse_quote!(::step_p21));
        let ans: syn::Type =
            syn::parse_str("Option<::step_p21::tables::PlaceHolder<THolder>>")
                .unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(place_holder), ans);

        let ty: syn::Type = syn::parse_str("Vec<T>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let place_holder = f
            .into_holder()
            .unwrap()
            .into_place_holder(&syn::parse_quote!(::step_p21));
        let ans: syn::Type =
            syn::parse_str("Vec<::step_p21::tables::PlaceHolder<THolder>>")
                .unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(place_holder), ans);

        let ty: syn::Type = syn::parse_str("Option<Vec<T>>").unwrap();
        let f: FieldType = ty.try_into().unwrap();
        let place_holder = f
            .into_holder()
            .unwrap()
            .into_place_holder(&syn::parse_quote!(::step_p21));
        let ans: syn::Type = syn::parse_str(
            "Option<Vec<::step_p21::tables::PlaceHolder<THolder>>>",
        )
        .unwrap();
        assert_eq!(<FieldType as Into<syn::Type>>::into(place_holder), ans);
    }
}
