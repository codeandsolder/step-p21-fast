use super::{
    super::{
        combinator::{EsprParser, ParseResult, alt, char, opt, tag, tuple},
        expression::numeric_expression,
        identifier::{entity_ref, type_ref},
    },
    simple_types,
};
use crate::ast::{Bound, Expression, Type};

/// 193 `concrete_types` = [`aggregation_types`] | [`simple_types`] |
/// [`type_ref`].
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn concrete_types(input: &str) -> ParseResult<'_, Type> {
    alt((
        aggregation_types,
        simple_types.map(Type::Simple),
        type_ref.map(Type::Named),
    ))
    .parse(input)
}

/// 172 `aggregation_types` = [`array_type`] | [`bag_type`] | [`list_type`] |
/// [`set_type`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn aggregation_types(input: &str) -> ParseResult<'_, Type> {
    alt((array_type, bag_type, list_type, set_type)).parse(input)
}

/// 175 `array_type` = ARRAY [`bound_spec`] OF \[ OPTIONAL \] \[ UNIQUE \]
/// [`instantiable_type`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn array_type(input: &str) -> ParseResult<'_, Type> {
    tuple((
        tag("ARRAY"),
        bound_spec,
        tag("OF"),
        opt(tag("OPTIONAL")),
        opt(tag("UNIQUE")),
        instantiable_type,
    ))
    .map(|(_set, bound, _of, optional, unique, base)| Type::Array {
        bound: Some(bound), // Maybe None for general_array_type
        unique: unique.is_some(),
        optional: optional.is_some(),
        base: Box::new(base),
    })
    .parse(input)
}

/// 180 `bag_type` = BAG \[ [`bound_spec`] \] OF [`instantiable_type`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn bag_type(input: &str) -> ParseResult<'_, Type> {
    tuple((tag("BAG"), opt(bound_spec), tag("OF"), instantiable_type))
        .map(|(_set, bound, _of, base)| Type::Bag {
            bound,
            base: Box::new(base),
        })
        .parse(input)
}

/// 250 `list_type` = LIST \[ [`bound_spec`] \] OF \[ UNIQUE \]
/// [`instantiable_type`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn list_type(input: &str) -> ParseResult<'_, Type> {
    tuple((
        tag("LIST"),
        opt(bound_spec),
        tag("OF"),
        opt(tag("UNIQUE")),
        instantiable_type,
    ))
    .map(|(_set, bound, _of, unique, base)| Type::List {
        bound,
        unique: unique.is_some(),
        base: Box::new(base),
    })
    .parse(input)
}

/// 303 `set_type` = SET \[ [`bound_spec`] \] OF [`instantiable_type`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn set_type(input: &str) -> ParseResult<'_, Type> {
    tuple((tag("SET"), opt(bound_spec), tag("OF"), instantiable_type))
        .map(|(_set, bound, _of, base)| Type::Set {
            bound,
            base: Box::new(base),
        })
        .parse(input)
}

/// 183 `bound_1` = [`numeric_expression`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn bound_1(input: &str) -> ParseResult<'_, Expression> {
    numeric_expression(input)
}

/// 184 `bound_2` = [`numeric_expression`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn bound_2(input: &str) -> ParseResult<'_, Expression> {
    numeric_expression(input)
}

/// 185 `bound_spec` = `[` [`bound_1`] `:` [`bound_2`] `]` .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn bound_spec(input: &str) -> ParseResult<'_, Bound> {
    tuple((char('['), bound_1, char(':'), bound_2, char(']')))
        .map(|(_open, lower, _comma, upper, _close)| Bound { lower, upper })
        .parse(input)
}

/// 240 `instantiable_type` = [`concrete_types`] | [`entity_ref`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn instantiable_type(input: &str) -> ParseResult<'_, Type> {
    alt((concrete_types, entity_ref.map(Type::Named))).parse(input)
}
