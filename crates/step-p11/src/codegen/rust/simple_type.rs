use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt, format_ident, quote};

use crate::ir::SimpleType;

impl ToTokens for SimpleType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use crate::ast::SimpleType::{
            Binary, Boolen, Integer, Logical, Number, Real, String_,
        };
        match self.0 {
            Number | Real => tokens.append(format_ident!("f64")),
            Integer => tokens.append(format_ident!("i64")),
            Logical => tokens.append_all(quote! { Logical }),
            Boolen => tokens.append(format_ident!("bool")),
            String_ { .. } => tokens.append(format_ident!("String")),
            Binary { .. } => tokens.append_all(quote! { Vec<u8> }),
        }
    }
}
