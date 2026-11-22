//! Parser for tokens defined in the table 2 of ISO-10303-21

use crate::{
    ast::{Name, Parameter, Uri},
    parser::{
        basic::{lower, upper},
        combinator::{ParseResult, char_, match_char, skip_spaces},
    },
};
use nom::{
    Parser,
    branch::alt,
    combinator::opt,
    error::{ErrorKind, ParseError},
};

use std::simd::{
    Simd,
    cmp::{SimdPartialEq, SimdPartialOrd},
};

/// sign = `+` | `-` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn sign(input: &str) -> ParseResult<'_, char> {
    alt((char_('+'), char_('-'))).parse(input)
}

/// integer = \[ [sign] \] [`digit`](super::basic::digit) {
/// [`digit`](super::basic::digit) } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn integer(input: &str) -> ParseResult<'_, i64> {
    let (rest, (sign, digits)) = number_prefix(input)?;
    integer_value(input, rest, sign, digits)
}

fn number_prefix(input: &str) -> ParseResult<'_, (Option<char>, &str)> {
    // A missing sign is the normal case, not a failed parser alternative.
    let (sign, rest) = match input.as_bytes().first() {
        Some(b'+') => (Some('+'), &input[1..]),
        Some(b'-') => (Some('-'), &input[1..]),
        _ => (None, input),
    };
    let (rest, digits) = required_digits(skip_spaces(rest))?;
    Ok((rest, (sign, digits)))
}

/// Split an ASCII digit prefix without decoding UTF-8 characters.
fn digit_span(input: &str) -> (&str, &str) {
    let count = input
        .as_bytes()
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .unwrap_or(input.len());
    let (digits, rest) = input.split_at(count);
    (rest, digits)
}

fn required_digits(input: &str) -> ParseResult<'_, &str> {
    let (rest, digits) = digit_span(input);
    if digits.is_empty() {
        Err(nom::Err::Error(ParseError::from_error_kind(
            input,
            ErrorKind::Digit,
        )))
    } else {
        Ok((rest, digits))
    }
}

fn integer_value<'a>(
    input: &'a str,
    rest: &'a str,
    sign: Option<char>,
    digits: &str,
) -> ParseResult<'a, i64> {
    parse_signed_integer(sign, digits)
        .map(|value| (rest, value))
        .map_err(|()| {
            nom::Err::Error(ParseError::from_error_kind(
                input,
                ErrorKind::MapRes,
            ))
        })
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

/// `E` \[ [sign] \] [`digit`](super::basic::digit) {
/// [`digit`](super::basic::digit) } .
fn exponent(input: &str) -> ParseResult<'_, i64> {
    let (rest, _) = match_char(input, 'E')?;
    let (rest, (sign, digits)) = number_prefix(skip_spaces(rest))?;
    integer_value(input, rest, sign, digits)
}

/// real = \[ [sign] \] [`digit`](super::basic::digit) {
/// [`digit`](super::basic::digit) } `.` { [`digit`](super::basic::digit) } \[
/// `E` \[ [sign] \] [`digit`](super::basic::digit) {
/// [`digit`](super::basic::digit) } \] .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn real(input: &str) -> ParseResult<'_, f64> {
    let (rest, (sign, integral)) = number_prefix(input)?;
    real_suffix(input, rest, sign, integral)
}

/// Parse the disjoint integer/real alternatives using one common digit scan.
pub(super) fn number(input: &str) -> ParseResult<'_, Parameter> {
    let (rest, (sign, integral)) = number_prefix(input)?;
    if rest.starts_with('.') {
        real_suffix(input, rest, sign, integral)
            .map(|(remaining, value)| (remaining, Parameter::Real(value)))
    } else {
        integer_value(input, rest, sign, integral)
            .map(|(remaining, value)| (remaining, Parameter::Integer(value)))
    }
}

fn real_suffix<'a>(
    input: &'a str,
    rest: &'a str,
    sign: Option<char>,
    integral: &str,
) -> ParseResult<'a, f64> {
    let (rest, _) = match_char(rest, '.')?;
    let (rest, fractional) = digit_span(rest);
    // Retain the optional exponent's backtracking semantics for malformed
    // exponents, without constructing a recoverable error when E is absent.
    let (rest, exp) = if rest.starts_with('E') {
        opt(exponent).parse(rest)?
    } else {
        (rest, None)
    };
    // Part 21's ordinary real-number spelling is already accepted directly
    // by f64::from_str. Borrow it from the input instead of allocating and
    // reparsing a formatted decimal string for every coordinate and vector.
    // Whitespace between sign/exponent components is also legal per this
    // parser's grammar; preserve the original normalization for those cases.
    let token = &input[..input.len() - rest.len()];
    if let Ok(number) = token.parse::<f64>() {
        return finite_real(rest, number);
    }
    let number = format!("{integral}.{fractional}e{}", exp.unwrap_or(0))
        .parse::<f64>()
        .map_err(|_| numeric_error(rest, "invalid-real"))?;
    finite_real(rest, if sign == Some('-') { -number } else { number })
}

/// Never turn an out-of-range STEP decimal into an infinite coordinate.
fn finite_real(rest: &str, number: f64) -> ParseResult<'_, f64> {
    if number.is_finite() {
        Ok((rest, number))
    } else {
        Err(numeric_error(rest, "real-out-of-range"))
    }
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
    let (mut rest, _) = match_char(input, '\'')?;
    let mut decoded = String::new();
    loop {
        // Search whole UTF-8 spans instead of building an intermediate
        // Vec<char> and speculatively parsing an escape at every character.
        let end = rest.find('\'').unwrap_or(rest.len());
        let (text, closing) = rest.split_at(end);
        let (after, _) = match_char(closing, '\'')?;
        decoded.push_str(text);
        if let Some(after_escape) = after.strip_prefix('\'') {
            decoded.push('\'');
            rest = after_escape;
        } else {
            return Ok((after, decoded));
        }
    }
}

/// resource = `<` `UNIVERSAL_RESOURCE_IDENTIFIER` `>` .
///
/// Parse as string, without validating as URI
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn resource(input: &str) -> ParseResult<'_, Uri> {
    anchor_name.map(Uri).parse(input)
}

/// enumeration = `.` [upper] { [upper] | [`digit`](super::basic::digit) } `.` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn enumeration(input: &str) -> ParseResult<'_, String> {
    (char_('.'), standard_keyword, char_('.'))
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

/// `entity_instance_name` = `#` ( [`digit`](super::basic::digit) ) {
/// [`digit`](super::basic::digit) } .
///
/// As discussed in ISO-10303-21 6.4.4.3 Entity instance names,
///
/// > NOTE 2 Leading zeros in entity instance names are ignored so "#001" is the
/// > same identifier as "#1".
///
/// leading zeros are ignored, and convert into `u64` type.
///
/// # Errors
/// Returns a parse error for invalid syntax or an identifier exceeding
/// [`u64::MAX`].
pub fn entity_instance_name(input: &str) -> ParseResult<'_, u64> {
    instance_name(input, '#')
}

/// `value_instance_name` = `@` ( [`digit`](super::basic::digit) ) {
/// [`digit`](super::basic::digit) } .
///
/// Leading zeros are ignored like as [`entity_instance_name`].
///
/// # Errors
/// Returns a parse error for invalid syntax or an identifier exceeding
/// [`u64::MAX`].
pub fn value_instance_name(input: &str) -> ParseResult<'_, u64> {
    instance_name(input, '@')
}

fn instance_name(input: &str, prefix: char) -> ParseResult<'_, u64> {
    let (digits, _) = match_char(input, prefix)?;
    let mut value = 0_u64;
    let mut count = 0;
    for &byte in digits.as_bytes() {
        let digit = byte.wrapping_sub(b'0');
        if digit > 9 {
            break;
        }
        let Some(next) = value
            .checked_mul(10)
            .and_then(|n| n.checked_add(u64::from(digit)))
        else {
            let (rest, _) = digit_span(&digits[count..]);
            return Err(u64_overflow(rest));
        };
        value = next;
        count += 1;
    }
    if count == 0 {
        return Err(nom::Err::Error(ParseError::from_error_kind(
            digits,
            ErrorKind::Digit,
        )));
    }
    Ok((&digits[count..], value))
}

/// `constant_entity_name` = `#` ( [upper] ) { [upper] |
/// [`digit`](super::basic::digit) } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn constant_entity_name(input: &str) -> ParseResult<'_, String> {
    (char_('#'), standard_keyword)
        .map(|(_sharp, name)| name)
        .parse(input)
}

/// `constant_value_name` = `@` ( [upper] ) { [upper] |
/// [`digit`](super::basic::digit) } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn constant_value_name(input: &str) -> ParseResult<'_, String> {
    (char_('@'), standard_keyword)
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
    if input.starts_with('@') {
        value_instance_name.map(Name::Value).parse(input)
    } else {
        entity_instance_name.map(Name::Entity).parse(input)
    }
}

/// `rhs_occurrence_name` = ( [`entity_instance_name`] | [`value_instance_name`]
/// | [`constant_entity_name`] | [`constant_value_name`]) .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn rhs_occurrence_name(input: &str) -> ParseResult<'_, Name> {
    match input.as_bytes() {
        [b'#', b'0'..=b'9', ..] => {
            entity_instance_name.map(Name::Entity).parse(input)
        }
        [b'@', b'0'..=b'9', ..] => {
            value_instance_name.map(Name::Value).parse(input)
        }
        [b'@', ..] => constant_value_name.map(Name::ConstantValue).parse(input),
        _ => constant_entity_name.map(Name::ConstantEntity).parse(input),
    }
}

/// `anchor_name` = `<` `URI_FRAGMENT_IDENTIFIER` `>` .
///
/// Parse as string, without validating as URI fragment identifier
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn anchor_name(input: &str) -> ParseResult<'_, String> {
    let (remaining, _) = match_char(input, '<')?;
    let end = remaining.find('>').unwrap_or(remaining.len());
    let (name, closing) = remaining.split_at(end);
    let (rest, _) = match_char(closing, '>')?;
    Ok((rest, name.to_owned()))
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

/// `standard_keyword` = [upper] { [upper] | [`digit`](super::basic::digit) } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn standard_keyword(input: &str) -> ParseResult<'_, String> {
    let (rest, keyword) = standard_keyword_span(input)?;
    Ok((rest, keyword.to_owned()))
}

/// Return the source span so both keyword forms allocate exactly once.
fn standard_keyword_span(input: &str) -> ParseResult<'_, &str> {
    if !matches!(input.as_bytes().first(), Some(b'A'..=b'Z' | b'_')) {
        return Err(nom::Err::Error(ParseError::from_error_kind(
            input,
            ErrorKind::Satisfy,
        )));
    }
    let rest = &input[1 + keyword_tail_len(&input.as_bytes()[1..])..];
    Ok((rest, &input[..input.len() - rest.len()]))
}

/// Length of the initial ASCII keyword continuation. Any non-ASCII byte
/// terminates the span, so the result is also a UTF-8 character boundary.
fn keyword_tail_len(input: &[u8]) -> usize {
    const LANES: usize = 16;
    let (chunks, tail) = input.as_chunks::<LANES>();
    for (index, &chunk) in chunks.iter().enumerate() {
        let bytes = Simd::from_array(chunk);
        // SIMD integer subtraction wraps. Subtracting the lower endpoint
        // converts each closed ASCII range into one unsigned comparison.
        let uppercase =
            (bytes - Simd::splat(b'A')).simd_le(Simd::splat(b'Z' - b'A'));
        let digits =
            (bytes - Simd::splat(b'0')).simd_le(Simd::splat(b'9' - b'0'));
        let valid = uppercase | digits | bytes.simd_eq(Simd::splat(b'_'));
        if let Some(lane) = (!valid).first_set() {
            return index * LANES + lane;
        }
    }
    input.len() - tail.len()
        + tail
            .iter()
            .position(|&byte| !matches!(byte, b'A'..=b'Z' | b'0'..=b'9' | b'_'))
            .unwrap_or(tail.len())
}

/// `user_defined_keyword` = `!` [upper] { [upper] |
/// [`digit`](super::basic::digit) } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn user_defined_keyword(input: &str) -> ParseResult<'_, String> {
    let (rest, _) = match_char(input, '!')?;
    let (rest, _) = standard_keyword_span(rest)?;
    // The leading '!' is part of the identifier. Removing it aliases a
    // user-defined type with a standard type of the same spelling.
    Ok((rest, input[..input.len() - rest.len()].to_owned()))
}

/// `tag_name` = ( [upper] | [lower] ) { [upper] | [lower] |
/// [`digit`](super::basic::digit) } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn tag_name(input: &str) -> ParseResult<'_, String> {
    let (rest, _) = alt((upper, lower)).parse(input)?;
    let (rest, _) = nom::bytes::complete::take_while(|c: char| {
        c.is_ascii_alphanumeric() || c == '_'
    })
    .parse(rest)?;
    Ok((rest, input[..input.len() - rest.len()].to_owned()))
}

/// `signature_content` = BASE64 .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn signature_content(input: &str) -> ParseResult<'_, String> {
    nom::bytes::complete::take_while1(|c: char| {
        c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=')
    })
    .map(str::to_owned)
    .parse(input)
}

#[cfg(test)]
mod tests {
    use nom::Finish;

    #[test]
    fn string_span_parser_handles_quotes_unicode_and_remaining_input() {
        for (input, value, residual) in [
            ("'';", "", ";"),
            ("'''';", "'", ";"),
            ("'abc''é😀''def'next", "abc'é😀'def", "next"),
            ("'é\n\\X2\\0000\\X0\\'tail", "é\n\\X2\\0000\\X0\\", "tail"),
        ] {
            assert_eq!(super::string(input), Ok((residual, value.to_owned())));
        }
        for input in ["", "no quote", "'", "'abc", "'abc''", "'''"] {
            assert!(super::string(input).is_err(), "{input:?}");
        }
    }

    #[test]
    fn borrowed_delimiter_scans_preserve_unicode_and_residuals() {
        assert_eq!(
            super::anchor_name("<é😀>suffix"),
            Ok(("suffix", "é😀".to_owned()))
        );
        assert_eq!(
            super::anchor_name("<>suffix"),
            Ok(("suffix", String::new()))
        );
        assert!(super::anchor_name("<é").is_err());
        assert!(super::resource("<é").is_err());
        assert_eq!(
            super::tag_name("_Ab9-foo"),
            Ok(("-foo", "_Ab9".to_owned()))
        );
        assert!(super::tag_name("9abc").is_err());
        assert_eq!(
            super::signature_content("aAZ9+/=;"),
            Ok((";", "aAZ9+/=".to_owned()))
        );
        assert!(super::signature_content(";").is_err());
    }

    #[test]
    fn keyword_scan_matches_scalar_for_every_byte_and_lane() {
        // Exercise unaligned slice starts, multiple complete vectors and
        // every possible tail length; the oracle is scalar byte matching.
        for offset in 0..16 {
            let mut storage = [b'A'; 64];
            let input = &mut storage[offset..offset + 49];
            for length in 0..=input.len() {
                assert_eq!(super::keyword_tail_len(&input[..length]), length);
                if let Some((last, _)) = input[..length].split_last_mut() {
                    *last = b'!';
                    assert_eq!(
                        super::keyword_tail_len(&input[..length]),
                        length - 1
                    );
                    input[length - 1] = b'A';
                }
            }
            for position in 0..input.len() {
                for byte in u8::MIN..=u8::MAX {
                    input[position] = byte;
                    let expected = input
                        .iter()
                        .position(|byte| !matches!(byte, b'A'..=b'Z' | b'0'..=b'9' | b'_'))
                        .unwrap_or(input.len());
                    assert_eq!(super::keyword_tail_len(input), expected);
                }
                input[position] = b'A';
            }
        }
    }

    #[test]
    fn keyword_scan_preserves_utf8_boundaries_and_initial_character_rule() {
        for length in 1..65 {
            let keyword = "A_9".repeat(length);
            let input = format!("{keyword}é");
            assert_eq!(
                super::standard_keyword(&input),
                Ok(("é", keyword.clone()))
            );
            let extension = format!("!{input}");
            assert_eq!(
                super::user_defined_keyword(&extension),
                Ok(("é", format!("!{keyword}")))
            );
        }
        for input in ["", "9A", "a", "é", "!9A"] {
            assert!(super::keyword(input).is_err(), "{input:?}");
        }
    }

    #[test]
    fn digit_span_matches_nom_for_all_byte_values_and_prefix_lengths() {
        use nom::character::complete::{digit0, digit1};
        use nom_language::error::VerboseError;

        for byte in u8::MIN..=u8::MAX {
            for length in 0..=40 {
                let input =
                    format!("{}{}0", "8".repeat(length), char::from(byte));
                let expected =
                    digit0::<_, VerboseError<_>>(input.as_str()).unwrap();
                assert_eq!(super::digit_span(&input), expected);
                assert_eq!(
                    super::required_digits(&input),
                    digit1::<_, VerboseError<_>>(input.as_str())
                );
            }
        }
        assert_eq!(super::digit_span(""), ("", ""));
        assert_eq!(super::digit_span("123٤"), ("٤", "123"));
    }

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
    fn real_exponents_that_overflow_f64_return_errors() {
        for input in ["1.0E309", "-1.0E400", "- 1.0E400"] {
            assert!(super::real(input).is_err(), "input: {input}");
        }
        let (_, finite) = super::real("1.7976931348623157E308").unwrap();
        assert!(finite.is_finite());
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
            ("!CUSTOM_1(", "(", "!CUSTOM_1"),
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
    fn user_defined_record_keyword_is_distinct_from_standard_keyword() {
        let user: crate::ast::Record = "!CUSTOM_1(1)".parse().unwrap();
        let standard: crate::ast::Record = "CUSTOM_1(1)".parse().unwrap();
        assert_eq!(user.name, "!CUSTOM_1");
        assert_eq!(standard.name, "CUSTOM_1");
        assert_ne!(user.name, standard.name);
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
    fn fused_identifier_scan_matches_digit_and_integer_oracle() {
        use nom::character::complete::digit1;
        use nom_language::error::VerboseError;

        for prefix in ['#', '@'] {
            for digits in [
                "",
                "0",
                "000",
                "9",
                "123456",
                "18446744073709551615",
                "18446744073709551616",
                "184467440737095516150",
                "99999999999999999999999999999999999999",
                "0000000000000000000000000000000000000000018446744073709551615",
                "0000000000000000000000000000000000000000018446744073709551616",
            ] {
                for suffix in ["", ";", "é", "٤", " ", "/*tail*/", "0"] {
                    let text = format!("{prefix}{digits}{suffix}");
                    let reference = digit1::<_, VerboseError<_>>(&text[1..]);
                    let expected = match reference {
                        Ok((rest, token)) => token
                            .parse::<u64>()
                            .map(|value| (rest, value))
                            .map_err(|_| super::u64_overflow(rest)),
                        Err(error) => Err(error),
                    };
                    assert_eq!(
                        super::instance_name(&text, prefix),
                        expected,
                        "{text:?}"
                    );
                }
            }
        }
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
