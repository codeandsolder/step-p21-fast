use crate::{
    ast::AttributeDecl,
    parser::{
        combinator::{EsprParser, ParseResult, alt, opt, tag, tuple},
        expression::{attribute_qualifier, group_qualifier},
        identifier::{attribute_id, attribute_ref},
    },
};

/// 177 `attribute_decl` = [`attribute_id`] | [`redeclared_attribute`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn attribute_decl(input: &str) -> ParseResult<'_, AttributeDecl> {
    alt((
        redeclared_attribute,
        attribute_id.map(AttributeDecl::Reference),
    ))
    .parse(input)
}

/// 280 `referenced_attribute` = [`attribute_ref`] | [`qualified_attribute`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn referenced_attribute(input: &str) -> ParseResult<'_, AttributeDecl> {
    alt((
        attribute_ref.map(AttributeDecl::Reference),
        qualified_attribute,
    ))
    .parse(input)
}

/// 275 `qualified_attribute` = SELF [`group_qualifier`] [`attribute_qualifier`]
/// .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn qualified_attribute(input: &str) -> ParseResult<'_, AttributeDecl> {
    tuple((tag("SELF"), group_qualifier, attribute_qualifier))
        .map(|(_self, group, attribute)| AttributeDecl::Qualified {
            group,
            attribute,
            rename: None,
        })
        .parse(input)
}

/// 279 `redeclared_attribute` = [`qualified_attribute`] \[ RENAMED
/// [`attribute_id`] \] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn redeclared_attribute(input: &str) -> ParseResult<'_, AttributeDecl> {
    tuple((
        qualified_attribute,
        opt(tuple((tag("RENAMED"), attribute_id))),
    ))
    .map(|(attr, opt)| match attr {
        AttributeDecl::Qualified {
            group,
            attribute,
            rename: _,
        } => AttributeDecl::Qualified {
            group,
            attribute,
            rename: opt.map(|(_renamed, name)| name),
        },
        _ => unreachable!(),
    })
    .parse(input)
}
