use crate::{
    ast::ReferenceEntry,
    parser::{
        combinator::{ParseResult, char_, many0_, tag_, tuple_},
        token::{lhs_occurrence_name, resource},
    },
};
use nom::Parser;

/// `reference_section` = `REFERENCE;` [`reference_list`] `ENDSEC;` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn reference_section(input: &str) -> ParseResult<'_, Vec<ReferenceEntry>> {
    tuple_((tag_("REFERENCE;"), reference_list, tag_("ENDSEC;")))
        .map(|(_start, list, _end)| list)
        .parse(input)
}

/// `reference_list` = { [`reference()`] } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn reference_list(input: &str) -> ParseResult<'_, Vec<ReferenceEntry>> {
    many0_(reference).parse(input)
}

/// reference = [`lhs_occurrence_name`] `=` [resource] `;` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn reference(input: &str) -> ParseResult<'_, ReferenceEntry> {
    tuple_((lhs_occurrence_name, char_('='), resource, char_(';')))
        .map(|(name, _def, resource, _semicolon)| ReferenceEntry {
            name,
            resource,
        })
        .parse(input)
}
