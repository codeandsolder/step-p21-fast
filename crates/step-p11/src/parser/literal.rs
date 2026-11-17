use super::{
    basis::{encoded_string_literal, simple_string_literal},
    combinator::{
        EsprParser, ParseResult, alt, not, peek, remarked, tag, tuple, value,
    },
};
use crate::ast::{Literal, Logical};

/// 251 literal = `binary_literal` | [`logical_literal`] | [`real_literal`] |
/// [`string_literal`] .
///
/// Integer value, e.g. `23`, will be recognized as a real number `23.0`.
/// Use [`integer_literal`] if you wish to parse it as an integer.
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn literal(input: &str) -> ParseResult<'_, Literal> {
    // FIXME binary_literal,
    alt((
        logical_literal.map(Literal::Logial),
        real_literal.map(Literal::Real),
        string_literal.map(Literal::String),
    ))
    .parse(input)
}

/// 255 `logical_literal` = `FALSE` | `TRUE` | `UNKNOWN` .
///
/// The keyword must be followed by something that cannot continue an
/// identifier, or `FALSE_POSITIVE` would tokenize as `FALSE` plus a stray
/// `_POSITIVE`. An EXPRESS `simple_id` continues with a letter, a digit **or an
/// underscore**, so all three have to be excluded -- guarding against letters
/// alone left identifiers like `TRUE_NORTH`, `TRUE2` and `UNKNOWN_9`
/// unparsable.
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn logical_literal(input: &str) -> ParseResult<'_, Logical> {
    tuple((
        alt((
            value(Logical::True, tag("TRUE")),
            value(Logical::False, tag("FALSE")),
            value(Logical::Unknown, tag("UNKNOWN")),
        )),
        peek(not(remarked(identifier_continuation))),
    ))
    .map(|(logical, _boundary)| logical)
    .parse(input)
}

/// One or more characters that may continue an EXPRESS `simple_id`.
///
/// A plain `fn` rather than a `take_while1` closure because the combinators it
/// feeds require `Clone`, which the closure is not.
fn identifier_continuation(
    input: &str,
) -> nom::IResult<&str, &str, nom::error::VerboseError<&str>> {
    nom::bytes::complete::take_while1(|c: char| c.is_alphanumeric() || c == '_')(
        input,
    )
}

/// 141 `integer_literal` = digits .
///
/// Negative integer, e.g. `-23`,
/// will be represented by the combination of `-` unary operator and integer
/// literal `23`
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn integer_literal(input: &str) -> ParseResult<'_, u64> {
    let (rest, (digits, remarks)) =
        remarked(nom::character::complete::digit1).parse(input)?;
    let number = digits.parse::<u64>().map_err(|_| {
        nom::Err::Failure(nom::error::VerboseError {
            errors: vec![(
                rest,
                nom::error::VerboseErrorKind::Context(
                    "integer-literal-overflow",
                ),
            )],
        })
    })?;
    Ok((rest, (number, remarks)))
}

/// 142 `real_literal` = `integer_literal` | ( digits `.` \[ digits \] \[ `e` \[
/// sign \] digits \] ) .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn real_literal(input: &str) -> ParseResult<'_, f64> {
    remarked(nom::number::complete::double).parse(input)
}

/// 310 `string_literal` = `simple_string_literal` | `encoded_string_literal` .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn string_literal(input: &str) -> ParseResult<'_, String> {
    alt((
        remarked(simple_string_literal),
        remarked(encoded_string_literal),
    ))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use nom::Finish;

    #[test]
    fn integer_literal() {
        let (residual, (value, _remarks)) =
            super::integer_literal("123").finish().unwrap();
        assert_eq!(value, 123);
        assert_eq!(residual, "");
    }

    #[test]
    fn real_literal() {
        let (residual, (value, _remarks)) =
            super::real_literal("123").finish().unwrap();
        assert_eq!(value, 123.0);
        assert_eq!(residual, "");

        let (residual, (value, _remarks)) =
            super::real_literal("123.456").finish().unwrap();
        assert_eq!(value, 123.456);
        assert_eq!(residual, "");

        let (residual, (value, _remarks)) =
            super::real_literal("1.23e-5").finish().unwrap();
        assert_eq!(value, 1.23e-5);
        assert_eq!(residual, "");
    }
}
