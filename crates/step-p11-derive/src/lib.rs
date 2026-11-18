use proc_macro::TokenStream;
use step_p11::{ast::SyntaxTree, codegen::rust::CratePrefix, ir::IR};

/// Compile inline EXPRESS into Rust code at the invocation site.
///
/// # Errors
/// Reports a compile-time diagnostic with the original string-literal span
/// when the EXPRESS grammar or semantic analysis fails.
///
/// ```
/// step_p11_derive::inline_express!(
///     r#"
/// SCHEMA explicit_draughting;
///   ENTITY a;
///     x: REAL;
///   END_ENTITY;
/// END_SCHEMA;
/// "#
/// );
/// ```
#[proc_macro]
pub fn inline_express(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::LitStr);
    let source = input.value();
    let st = match SyntaxTree::parse(&source) {
        Ok(st) => st,
        Err(error) => {
            return syn::Error::new(
                input.span(),
                format!("EXPRESS syntax error: {error:?}"),
            )
            .into_compile_error()
            .into();
        }
    };
    let ir = match IR::from_syntax_tree(&st) {
        Ok(ir) => ir,
        Err(error) => {
            return syn::Error::new(
                input.span(),
                format!("EXPRESS semantic error: {error}"),
            )
            .into_compile_error()
            .into();
        }
    };
    ir.to_token_stream(CratePrefix::External).into()
}
