//! Parse the associated attribute `#[holder(...)]` with `#[derive(Holder)]`
//!
//! There are three options:
//!
//! - `#[holder(table = {path::to::table::struct})]`
//! - `#[holder(field = {field_ident})]`
//! - `#[holder(use_place_holder)]`
//! - `#[holder(generate_deserialize)]`
//! - `#[holder(eq)]` to derive `Eq` on the generated holder

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolderAttr {
    pub table: Option<syn::Path>,
    pub field: Option<syn::Member>,
    pub place_holder: bool,
    pub generate_deserialize: bool,
    pub derive_eq: bool,
}

impl HolderAttr {
    pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut table = None;
        let mut field = None;
        let mut place_holder = false;
        let mut generate_deserialize = false;
        let mut derive_eq = false;

        for attr in attrs {
            // Only read `#[holder(...)]`
            if let Some(ident) = attr.meta.path().get_ident() {
                if ident != "holder" {
                    continue;
                }
            } else {
                continue;
            }

            match attr.parse_args().map_err(|error: syn::Error| {
                syn::Error::new(
                    error.span(),
                    format!("invalid #[holder] attribute: {error}"),
                )
            })? {
                Attr::Table(path) => {
                    table = Some(path);
                }
                Attr::Field(ident) => {
                    field = Some(ident);
                }
                Attr::PlaceHolder => {
                    place_holder = true;
                }
                Attr::GenerateDeserialize => {
                    generate_deserialize = true;
                }
                Attr::Eq => {
                    derive_eq = true;
                }
            }
        }
        Ok(Self {
            table,
            field,
            place_holder,
            generate_deserialize,
            derive_eq,
        })
    }
}

#[derive(Debug, PartialEq)]
enum Attr {
    Table(syn::Path),
    Field(syn::Member),
    PlaceHolder,
    GenerateDeserialize,
    Eq,
}

impl syn::parse::Parse for Attr {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        match ident.to_string().as_str() {
            "table" => {
                let _eq: syn::Token![=] = input.parse()?;
                let path = input.parse()?;
                Ok(Self::Table(path))
            }
            "field" => {
                let _eq: syn::Token![=] = input.parse()?;
                let member = input.parse()?;
                Ok(Self::Field(member))
            }
            "use_place_holder" => Ok(Self::PlaceHolder),
            "generate_deserialize" => Ok(Self::GenerateDeserialize),
            "eq" => Ok(Self::Eq),
            _ => Err(syn::parse::Error::new(
                ident.span(),
                "expected `table`, `field`, `eq`, `generate_deserialize`, or `use_place_holder`",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_attr_table() {
        let attr: Attr = syn::parse_str("table = Tables").unwrap();
        assert_eq!(attr, Attr::Table(syn::parse_str("Tables").unwrap()));

        let attr: Attr =
            syn::parse_str("table = ::some::path::to::Tables").unwrap();
        assert_eq!(
            attr,
            Attr::Table(syn::parse_str("::some::path::to::Tables").unwrap())
        );

        // table must take path
        assert!(syn::parse_str::<Attr>("table").is_err());
        // path cannot be empty
        assert!(syn::parse_str::<Attr>("table =").is_err());
    }

    #[test]
    fn parse_attr_field() {
        let attr: Attr = syn::parse_str("field = a").unwrap();
        assert_eq!(attr, Attr::Field(syn::parse_str("a").unwrap()));

        let attr: Attr = syn::parse_str("field = 1").unwrap();
        assert_eq!(attr, Attr::Field(syn::parse_str("1").unwrap()));

        // field cannot accept path
        assert!(syn::parse_str::<Attr>("field = ::some::path").is_err());
        // field must take an identifier or a positional index
        assert!(syn::parse_str::<Attr>("field").is_err());
        // identifier is empty
        assert!(syn::parse_str::<Attr>("field =").is_err());
    }

    #[test]
    fn parse_attr_eq() {
        assert_eq!(syn::parse_str::<Attr>("eq").unwrap(), Attr::Eq);
        assert!(syn::parse_str::<Attr>("eq = true").is_err());

        let attr: syn::Attribute = syn::parse_quote!(#[holder(eq)]);
        assert!(HolderAttr::parse(&[attr]).unwrap().derive_eq);
    }

    #[test]
    fn parse_attr_place_holder() {
        let attr: Attr = syn::parse_str("use_place_holder").unwrap();
        assert_eq!(attr, Attr::PlaceHolder);

        assert!(syn::parse_str::<Attr>("use_place_holder = true").is_err());
        // typo
        assert!(syn::parse_str::<Attr>("use_place_helder").is_err());
    }
}
