//! Parser for tokens defined in the table 2 of ISO-10303-21

use crate::{
    ast::{Name, Uri},
    parser::{
        basic::{digit, lower, upper},
        combinator::{ParseResult, tuple},
    },
};
use nom::{
    Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{
        char, digit0, digit1, multispace0, none_of, satisfy,
    },
    combinator::{map, map_res, opt},
    multi::{many0, many1},
};

/// sign = `+` | `-` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn sign(input: &str) -> ParseResult<'_, char> {
    alt((char('+'), char('-'))).parse(input)
}

/// integer = \[ [sign] \] [digit] { [digit] } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn integer(input: &str) -> ParseResult<'_, i64> {
    map_res(
        tuple((opt(sign), multispace0, digit1)),
        |(sign, _, digits)| parse_signed_integer(sign, digits),
    )
    .parse(input)
}

/// Parse the full i64 range without allocating a signed intermediate string.
fn parse_signed_integer(sign: Option<char>, digits: &str) -> Result<i64, ()> {
    let magnitude = digits.parse::<u64>().map_err(|_| ())?;
    if sign == Some('-') {
        if magnitude == i64::MAX as u64 + 1 {
            Ok(i64::MIN)
        } else {
            i64::try_from(magnitude).map(|value| -value).map_err(|_| ())
        }
    } else {
        i64::try_from(magnitude).map_err(|_| ())
    }
}

/// `E` \[ [sign] \] [digit] { [digit] } .
fn exponent(input: &str) -> ParseResult<'_, i64> {
    map_res(
        tuple((char('E'), multispace0, opt(sign), multispace0, digit1)),
        |(_e, _sp1, sign, _sp2, digits)| parse_signed_integer(sign, digits),
    )
    .parse(input)
}

/// real = \[ [sign] \] [digit] { [digit] } `.` { [digit] } \[ `E` \[ [sign] \]
/// [digit] { [digit] } \] .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn real(input: &str) -> ParseResult<'_, f64> {
    let (rest, (sign, _space, integral, _point, fractional, exp)) = tuple((
        opt(sign),
        multispace0,
        digit1,
        char('.'),
        digit0,
        opt(exponent),
    ))
    .parse(input)?;
    // Part 21's ordinary real-number spelling is already accepted directly
    // by f64::from_str. Borrow it from the input instead of allocating and
    // reparsing a formatted decimal string for every coordinate and vector.
    // Whitespace between sign/exponent components is also legal per this
    // parser's grammar; preserve the original normalization for those cases.
    let token = &input[..input.len() - rest.len()];
    if let Ok(number) = token.parse::<f64>() {
        return Ok((rest, number));
    }
    let number = format!("{integral}.{fractional}e{}", exp.unwrap_or(0))
        .parse::<f64>()
        .map_err(|_| numeric_error(rest, "invalid-real"))?;
    Ok((rest, if sign == Some('-') { -number } else { number }))
}

/// Parse a STEP string, including its doubled-apostrophe escape syntax.
///
/// EXPRESS string-token grammar:
///
/// ```text
/// string = "'" { [special] | [digit] | [space] | [lower] | [upper] |
///   high_codepoint | [apostrophe] [apostrophe] |
///   [reverse_solidus] [reverse_solidus] | control_directive } "'" .
/// ```
///
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn string(input: &str) -> ParseResult<'_, String> {
    let escaped_char = map(tag("''"), |_| '\''); // Parse '' as a single '
    let normal_char = none_of("'"); // Parse any character except '

    let string_content = many0(escaped_char.or(normal_char.map(|c| c)));

    tuple((char('\''), string_content, char('\'')))
        .map(|(_start, s, _end)| s.iter().collect())
        .parse(input)
}

/// resource = `<` `UNIVERSAL_RESOURCE_IDENTIFIER` `>` .
///
/// Parse as string, without validating as URI
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn resource(input: &str) -> ParseResult<'_, Uri> {
    tuple((char('<'), many0(none_of(">")), char('>')))
        .map(|(_start, s, _end)| Uri(s.iter().collect()))
        .parse(input)
}

/// enumeration = `.` [upper] { [upper] | [digit] } `.` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn enumeration(input: &str) -> ParseResult<'_, String> {
    tuple((char('.'), standard_keyword, char('.')))
        .map(|(_head, name, _tail)| name)
        .parse(input)
}

// Root error for u64 overflow
//
// FIXME Though it works, should we use `VerboseErrorKind::Context` for this
// usage?
fn numeric_error<'a>(
    input: &'a str,
    reason: &'static str,
) -> nom::Err<nom_language::error::VerboseError<&'a str>> {
    nom::Err::Failure(nom_language::error::VerboseError {
        errors: vec![(
            input,
            nom_language::error::VerboseErrorKind::Context(reason),
        )],
    })
}

fn u64_overflow(
    input: &str,
) -> nom::Err<nom_language::error::VerboseError<&str>> {
    numeric_error(input, "u64-overflow")
}

/// `entity_instance_name` = `#` ( [digit] ) { [digit] } .
///
/// As discussed in ISO-10303-21 6.4.4.3 Entity instance names,
///
/// > NOTE 2 Leading zeros in entity instance names are ignored so "#001" is the
/// > same identifier as "#1".
///
/// leading zeros are ignored, and convert into `u64` type.
///
/// Error
/// -------
/// - FIXME: If the input cannot be represented by `u64`, i.e. larger than
///   [`u64::MAX`]
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn entity_instance_name(input: &str) -> ParseResult<'_, u64> {
    let (input, name) = tuple((char('#'), digit1))
        .map(|(_sharp, name): (_, &str)| name.parse())
        .parse(input)?;
    name.map_or_else(|_| Err(u64_overflow(input)), |value| Ok((input, value)))
}

/// `value_instance_name` = `@` ( [digit] ) { [digit] } .
///
/// Leading zeros are ignored like as [`entity_instance_name`].
///
/// Error
/// -------
/// - FIXME: If the input cannot be represented by `u64`, i.e. larger than
///   [`u64::MAX`]
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn value_instance_name(input: &str) -> ParseResult<'_, u64> {
    let (input, name) = tuple((char('@'), digit1))
        .map(|(_sharp, name): (_, &str)| name.parse())
        .parse(input)?;
    name.map_or_else(|_| Err(u64_overflow(input)), |value| Ok((input, value)))
}

/// `constant_entity_name` = `#` ( [upper] ) { [upper] | [digit] } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn constant_entity_name(input: &str) -> ParseResult<'_, String> {
    tuple((char('#'), standard_keyword))
        .map(|(_sharp, name)| name)
        .parse(input)
}

/// `constant_value_name` = `@` ( [upper] ) { [upper] | [digit] } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn constant_value_name(input: &str) -> ParseResult<'_, String> {
    tuple((char('@'), standard_keyword))
        .map(|(_sharp, name)| name)
        .parse(input)
}

/// `lhs_occurrence_name` = ( [`entity_instance_name`] | [`value_instance_name`]
/// ) .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn lhs_occurrence_name(input: &str) -> ParseResult<'_, Name> {
    alt((
        entity_instance_name.map(Name::Entity),
        value_instance_name.map(Name::Value),
    ))
    .parse(input)
}

/// `rhs_occurrence_name` = ( [`entity_instance_name`] | [`value_instance_name`]
/// | [`constant_entity_name`] | [`constant_value_name`]) .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn rhs_occurrence_name(input: &str) -> ParseResult<'_, Name> {
    alt((
        entity_instance_name.map(Name::Entity),
        value_instance_name.map(Name::Value),
        constant_entity_name.map(Name::ConstantEntity),
        constant_value_name.map(Name::ConstantValue),
    ))
    .parse(input)
}

/// `anchor_name` = `<` `URI_FRAGMENT_IDENTIFIER` `>` .
///
/// Parse as string, without validating as URI fragment identifier
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn anchor_name(input: &str) -> ParseResult<'_, String> {
    tuple((char('<'), many0(none_of(">")), char('>')))
        .map(|(_start, s, _end)| s.iter().collect())
        .parse(input)
}

/// keyword = [`user_defined_keyword`] | [`standard_keyword`] .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn keyword(input: &str) -> ParseResult<'_, String> {
    // An extension keyword is syntactically unambiguous at the first byte.
    // Avoid constructing a recoverable error for every ordinary keyword.
    if input.starts_with('!') {
        user_defined_keyword(input)
    } else {
        standard_keyword(input)
    }
}

/// `standard_keyword` = [upper] { [upper] | [digit] } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn standard_keyword(input: &str) -> ParseResult<'_, String> {
    let (rest, _) = upper(input)?;
    let (rest, _) = nom::bytes::complete::take_while(
        |c: char| matches!(c, 'A'..='Z' | '_' | '0'..='9'),
    )
    .parse(rest)?;
    // Preserve the exact identifier (including underscores) with one string
    // allocation, instead of allocating Vec<char> and then another String.
    Ok((rest, input[..input.len() - rest.len()].to_owned()))
}

/// `user_defined_keyword` = `!` [upper] { [upper] | [digit] } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn user_defined_keyword(input: &str) -> ParseResult<'_, String> {
    tuple((char('!'), standard_keyword))
        .map(|(_e, name)| name)
        .parse(input)
}

/// `tag_name` = ( [upper] | [lower] ) { [upper] | [lower] | [digit] } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn tag_name(input: &str) -> ParseResult<'_, String> {
    tuple((alt((upper, lower)), many0(alt((upper, lower, digit)))))
        .map(|(first, tail)| {
            let head = &[first];
            head.iter().chain(tail.iter()).collect()
        })
        .parse(input)
}

/// `signature_content` = BASE64 .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn signature_content(input: &str) -> ParseResult<'_, String> {
    let base_char = satisfy(
        |c| matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '+' | '/' | '='),
    );
    many1(base_char)
        .map(|chars| chars.iter().collect())
        .parse(input)
}

#[cfg(test)]
mod tests {
    use nom::Finish;

    #[test]
    fn real() {
        let (res, s) = super::real("1.23").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(s, 1.23);

        let (res, s) = super::real("1.23E4").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(s, 1.23e4);

        let (res, s) = super::real("1.23E-4").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(s, 1.23e-4);

        let (res, s) = super::real("-1.23E4").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(s, -1.23e4);

        let (res, s) = super::real("-1.23E-4").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(s, -1.23e-4);

        assert!(super::real("123").finish().is_err());
    }

    #[test]
    fn integer_boundaries_and_overflow_are_errors_not_panics() {
        for (input, expected) in [
            ("0", 0),
            ("+42", 42),
            ("-42", -42),
            ("- 42", -42),
            ("9223372036854775807", i64::MAX),
            ("-9223372036854775808", i64::MIN),
        ] {
            let (rest, parsed) = super::integer(input).expect("valid integer");
            assert_eq!(rest, "", "input: {input}");
            assert_eq!(parsed, expected, "input: {input}");
        }
        for input in [
            "9223372036854775808",
            "-9223372036854775809",
            "18446744073709551616",
            "-18446744073709551616",
        ] {
            assert!(super::integer(input).is_err(), "input: {input}");
        }
        assert!(super::exponent("E9223372036854775808").is_err());
    }

    #[test]
    fn keyword_tokenization_preserves_extension_and_boundary_behavior() {
        for (input, rest, expected) in [
            (
                "B_SPLINE_SURFACE_WITH_KNOTS012(foo)",
                "(foo)",
                "B_SPLINE_SURFACE_WITH_KNOTS012",
            ),
            ("A0_", "", "A0_"),
            ("!CUSTOM_1(", "(", "CUSTOM_1"),
            ("_1_2.", ".", "_1_2"),
        ] {
            let (remainder, parsed) = super::keyword(input).unwrap();
            assert_eq!(remainder, rest, "input: {input}");
            assert_eq!(parsed, expected, "input: {input}");
        }
        assert!(super::standard_keyword("0KEY").is_err());
        assert!(super::standard_keyword("lower").is_err());
        assert!(super::keyword("!0NAME").is_err());
    }

    #[test]
    fn real_fast_path_preserves_bits_and_whitespace_fallback() {
        for (token, expected) in [
            ("1.23", 1.23_f64),
            ("1.23E4", 1.23e4),
            ("-1.23E-4", -1.23e-4),
            ("+12.", 12.0),
            ("-0.0", -0.0),
            ("0.000000000000000000000000000005", 5e-30),
            ("1.25E  -  03", 1.25e-3),
            ("-  2.5", -2.5),
        ] {
            let (remainder, parsed) = super::real(token).unwrap();
            assert_eq!(remainder, "", "token: {token}");
            assert_eq!(parsed.to_bits(), expected.to_bits(), "token: {token}");
        }
    }

    #[test]
    fn string() {
        let (res, s) = super::string("'vim'").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(s, "vim");
    }

    #[test]
    fn escaped_string() {
        let (res, s) = super::string("'vim''s'").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(s, "vim's");
    }

    #[test]
    fn instance_name() {
        let (res, s) = super::entity_instance_name(
            "#18446744073709551615", /* u64::MAX */
        )
        .finish()
        .unwrap();
        assert_eq!(res, "");
        assert_eq!(s, u64::MAX);

        let (res, s) = super::value_instance_name(
            "@18446744073709551615", /* u64::MAX */
        )
        .finish()
        .unwrap();
        assert_eq!(res, "");
        assert_eq!(s, u64::MAX);

        // u64 overflow
        assert!(
            super::entity_instance_name(
                "#18446744073709551616" /* u64::MAX + 1 */
            )
            .finish()
            .is_err()
        );
        assert!(
            super::value_instance_name(
                "@18446744073709551616" /* u64::MAX + 1 */
            )
            .finish()
            .is_err()
        );

        // zeros should be ignored
        let (res, s) = super::entity_instance_name("#001").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(s, 1);
        let (res, s) = super::value_instance_name("@001").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(s, 1);
    }
}
