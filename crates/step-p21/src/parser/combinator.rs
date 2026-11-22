//! Parser combinators extended for STEP exchange structure
//!
//! This is helper submodule for writting a parser like as WSN definitions.
//!
//! Token separators in exchange structure is one of
//!
//! - space
//! - explicit print control directives (`\N\` and `\F\` )
//! - comments
//!
//! and combinators in this submodule responsible for handling them.

use nom::{
    IResult, Parser,
    bytes::complete::tag,
    combinator::opt,
    error::{ErrorKind, ParseError},
};
use nom_language::error::VerboseError;

/// Parse result
pub type ParseResult<'a, X> = IResult<&'a str, X, VerboseError<&'a str>>;

/// Alias of `nom::Parser`
pub trait ExchangeParser<'a, X>:
    Clone + nom::Parser<&'a str, Output = X, Error = VerboseError<&'a str>>
{
}

impl<'a, X, T> ExchangeParser<'a, X> for T where
    T: Clone + nom::Parser<&'a str, Output = X, Error = VerboseError<&'a str>>
{
}

#[must_use = "the returned parser must be invoked to match the character"]
pub fn char_<'a>(c: char) -> impl ExchangeParser<'a, char> {
    move |input| match_char(input, c)
}

/// Match a fixed character without decoding a distinct leading character.
pub(super) fn match_char(input: &str, expected: char) -> ParseResult<'_, char> {
    input
        .strip_prefix(expected)
        .map(|rest| (rest, expected))
        .ok_or_else(|| nom::Err::Error(ParseError::from_char(input, expected)))
}

#[must_use = "the returned parser must be invoked to match the tag"]
pub fn tag_<'a>(name: &'static str) -> impl ExchangeParser<'a, &'a str> {
    move |input| {
        let (input, c) = nom::bytes::complete::tag(name).parse(input)?;
        Ok((input, c))
    }
}

pub fn opt_<'a, O>(
    f: impl ExchangeParser<'a, O>,
) -> impl ExchangeParser<'a, Option<O>> {
    move |input| {
        let (input, c) = nom::combinator::opt(f.clone()).parse(input)?;
        Ok((input, c))
    }
}

/// Comment
///
/// A comment shall be encoded as a solidus asterisk `/*`
/// followed by any number of characters from the basic alphabet,
/// and terminated by an asterisk solidus `*/`
///
/// These comments are dropped while parsing. Do not passed to following convert
/// step.
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn comment(input: &str) -> ParseResult<'_, String> {
    comment_slice(input).map(|(remaining, text)| (remaining, text.to_owned()))
}

/// Recognize a comment without materializing its body. Most callers only
/// discard comments while separating tokens, including large STEP fixtures.
fn comment_slice(input: &str) -> ParseResult<'_, &str> {
    let (remaining, _) = tag("/*")(input)?;
    comment_body(remaining).ok_or_else(|| {
        nom::Err::Error(ParseError::from_error_kind(
            remaining,
            ErrorKind::TakeUntil,
        ))
    })
}

/// Split the bytes after a comment opener at its first closing delimiter.
fn comment_body(content: &str) -> Option<(&str, &str)> {
    let end = content.find("*/")?;
    Some((&content[end + 2..], &content[..end]))
}

/// Comments with front/back spaces, or multi-space at least 1 char
///
/// - This never matches to empty string.
/// - Drop matched comments and spaces
///
/// FIXME
/// ------
/// - support explicit print control directives
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn separator(input: &str) -> ParseResult<'_, ()> {
    let rest = ignorable(input);
    if rest.len() == input.len() {
        Err(nom::Err::Error(ParseError::from_error_kind(
            input,
            ErrorKind::MultiSpace,
        )))
    } else {
        Ok((rest, ()))
    }
}

pub fn many0_<'a, O>(
    f: impl ExchangeParser<'a, O>,
) -> impl ExchangeParser<'a, Vec<O>> {
    move |input| {
        let (input, first) = match opt(f.clone()).parse(input)? {
            (input, Some(first)) => (input, first),
            (input, None) => return Ok((input, Vec::new())),
        };
        repeat_after(input, first, f.clone(), None)
    }
}

pub fn many1_<'a, O>(
    f: impl ExchangeParser<'a, O>,
) -> impl ExchangeParser<'a, Vec<O>> {
    move |input| {
        let (input, first) = f.clone().parse(input)?;
        repeat_after(input, first, f.clone(), None)
    }
}

/// Collect one or more elements separated by optional STEP separators.
/// A recoverable tail error leaves the entire speculative tail unconsumed.
fn repeat_after<'a, O>(
    mut input: &'a str,
    first: O,
    mut element: impl ExchangeParser<'a, O>,
    delimiter: Option<char>,
) -> ParseResult<'a, Vec<O>> {
    // Singleton separated lists need only one slot. Larger lists use the
    // normal Vec growth policy to avoid an immediate reallocation.
    let Some(mut start) = following_start(input, delimiter) else {
        return Ok((input, vec![first]));
    };
    let mut values = Vec::new();
    values.extend([first]);
    loop {
        match element.parse(start) {
            Ok((rest, value)) => {
                if rest.len() == input.len() {
                    return Err(nom::Err::Error(ParseError::from_error_kind(
                        input,
                        ErrorKind::Many0,
                    )));
                }
                values.push(value);
                input = rest;
            }
            Err(nom::Err::Error(_)) => return Ok((input, values)),
            Err(error) => return Err(error),
        }
        let Some(next) = following_start(input, delimiter) else {
            return Ok((input, values));
        };
        start = next;
    }
}

/// Locate the next element without allocating a speculative delimiter error.
fn following_start(input: &str, delimiter: Option<char>) -> Option<&str> {
    let start = ignorable(input);
    delimiter.map_or(Some(start), |delimiter| {
        start.strip_prefix(delimiter).map(ignorable)
    })
}

/// Skip the ASCII whitespace recognized by STEP's existing grammar.
pub(super) fn skip_spaces(input: &str) -> &str {
    let skipped = input
        .as_bytes()
        .iter()
        .position(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        .unwrap_or(input.len());
    // Every skipped byte is ASCII, so this is a UTF-8 boundary.
    &input[skipped..]
}

/// Skip zero or more whitespace characters and complete comments.
/// An unfinished comment remains for the following grammar production
/// to report as invalid.
#[must_use = "returns the remaining input after skipping separators"]
pub fn ignorable(mut input: &str) -> &str {
    loop {
        match input.as_bytes() {
            [b' ' | b'\t' | b'\r' | b'\n', ..] => {
                input = &input[1..];
            }
            [b'/', b'*', ..] => {
                let Some((rest, _)) = comment_body(&input[2..]) else {
                    return input;
                };
                input = rest;
            }
            _ => return input,
        }
    }
}

pub fn separated<'a, O>(
    c: char,
    f: impl ExchangeParser<'a, O>,
) -> impl ExchangeParser<'a, Vec<O>> {
    move |input| {
        let (input, first) = f.clone().parse(input)?;
        repeat_after(input, first, f.clone(), Some(c))
    }
}

pub fn comma_separated<'a, O>(
    f: impl ExchangeParser<'a, O>,
) -> impl ExchangeParser<'a, Vec<O>> {
    separated(',', f)
}

/// Sequence of separated tokens
pub fn tuple_<'a, O, List: Tuple<'a, O>>(
    mut l: List,
) -> impl ExchangeParser<'a, O> {
    move |input| l.parse(input)
}

/// helper for [tuple_]
pub trait Tuple<'a, O>: Clone {
    ///
    /// # Errors
    /// Returns a parse error when the input does not satisfy the corresponding
    /// STEP grammar production.
    fn parse(&mut self, input: &'a str) -> ParseResult<'a, O>;
}

/// Parse a sequence of grammar productions, skipping separators between them.
macro_rules! parse_tuple {
    ($input:ident; $first:ident => $first_out:ident $(, $parser:ident => $output:ident)*) => {
        let ($input, $first_out) = $first.clone().parse($input)?;
        $(
            let $input = ignorable($input);
            let ($input, $output) = $parser.clone().parse($input)?;
        )*
    };
}

macro_rules! impl_tuple {
    ($($F:ident),*; $($O:ident),*; $($f:ident),*; $($o:ident),*) => {
        impl<'a, $($F),*, $($O),*> Tuple<'a, ($($O),*)> for ($($F),*)
        where
            $( $F: ExchangeParser<'a, $O> ),*
        {
            fn parse(&mut self, input: &'a str) -> ParseResult<'a, ($($O),*)> {
                let ($($f),*) = self;
                parse_tuple!(input; $($f => $o),*);
                Ok((input, ($($o),*)))
            }
        }
    };
}

impl_tuple!(
    F1, F2;
    O1, O2;
    f1, f2;
    o1, o2
);
impl_tuple!(
    F1, F2, F3;
    O1, O2, O3;
    f1, f2, f3;
    o1, o2, o3
);
impl_tuple!(
    F1, F2, F3, F4;
    O1, O2, O3, O4;
    f1, f2, f3, f4;
    o1, o2, o3, o4
);
impl_tuple!(
    F1, F2, F3, F4, F5;
    O1, O2, O3, O4, O5;
    f1, f2, f3, f4, f5;
    o1, o2, o3, o4, o5
);
impl_tuple!(
    F1, F2, F3, F4, F5, F6;
    O1, O2, O3, O4, O5, O6;
    f1, f2, f3, f4, f5, f6;
    o1, o2, o3, o4, o5, o6
);
impl_tuple!(
    F1, F2, F3, F4, F5, F6, F7;
    O1, O2, O3, O4, O5, O6, O7;
    f1, f2, f3, f4, f5, f6, f7;
    o1, o2, o3, o4, o5, o6, o7
);
impl_tuple!(
    F1, F2, F3, F4, F5, F6, F7, F8;
    O1, O2, O3, O4, O5, O6, O7, O8;
    f1, f2, f3, f4, f5, f6, f7, f8;
    o1, o2, o3, o4, o5, o6, o7, o8
);
impl_tuple!(
    F1, F2, F3, F4, F5, F6, F7, F8, F9;
    O1, O2, O3, O4, O5, O6, O7, O8, O9;
    f1, f2, f3, f4, f5, f6, f7, f8, f9;
    o1, o2, o3, o4, o5, o6, o7, o8, o9
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::basic::digit;
    use nom::Finish;

    #[test]
    fn separator_scanner_matches_original_combinators() {
        use nom::{
            branch::alt,
            bytes::complete::{tag, take_until},
            character::complete::{multispace0, multispace1},
            combinator::value,
            multi::many1,
        };

        fn reference_comment(input: &str) -> ParseResult<'_, &str> {
            let (rest, _) = tag("/*")(input)?;
            let (rest, text) = take_until("*/").parse(rest)?;
            let (rest, _) = tag("*/")(rest)?;
            Ok((rest, text))
        }

        for prefix in
            ["", " ", "\t\n\r", "/*é*/", "/*a*//*b*/", "  /* 🦀 */ \r\n"]
        {
            for suffix in [
                "",
                "A",
                "/*unterminated",
                "/* */ /*next*/é",
                "/",
                " //",
                "\u{3000}",
            ] {
                let input = format!("{prefix}{suffix}");
                let comments =
                    many1((multispace0, reference_comment, multispace0))
                        .map(|_| ());
                let expected: ParseResult<'_, ()> =
                    alt((comments, value((), multispace1))).parse(&input);
                let actual = super::separator(&input);
                match (actual, expected) {
                    (Err(nom::Err::Error(_)), Err(nom::Err::Error(_))) => {}
                    (actual, expected) => {
                        assert_eq!(actual, expected, "{input:?}")
                    }
                }
            }
        }
    }

    #[test]
    fn direct_character_match_agrees_with_nom_including_unicode() {
        for expected in ['A', '_', '#', '\'', '\0', 'é', '🦀', '\u{3000}'] {
            for input in
                ["", "A", "#{}", "éA", "🦀!", "\u{3000}a", "\0tail", "Aé"]
            {
                assert_eq!(
                    super::match_char(input, expected),
                    nom::character::complete::char::<_, VerboseError<_>>(
                        expected
                    )(input)
                );
            }
        }
    }

    #[test]
    fn separated_accepts_unicode_delimiters_and_restores_failed_tail() {
        let mut parser = super::separated('🦀', digit);
        assert_eq!(
            parser.parse("1 /*a*/ 🦀 2 🦀 X"),
            Ok((" 🦀 X", vec!['1', '2']))
        );
        assert_eq!(
            parser.parse("1   /*tail*/"),
            Ok(("   /*tail*/", vec!['1']))
        );
    }

    #[test]
    fn comment() {
        let (res, c) = super::comment("/*🦀*/").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(c, "🦀");

        let (res, c) = super::comment("/* vim * vim */").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(c, " vim * vim ");
    }

    #[test]
    fn comment_handles_empty_unicode_stars_and_unterminated_input() {
        for (input, content, rest) in [
            ("/**/", "", ""),
            ("/*data**with*stars*/NEXT", "data**with*stars", "NEXT"),
            ("/* 🌍 */ end", " 🌍 ", " end"),
            ("/*/*/", "/", ""),
        ] {
            let (remaining, parsed) = super::comment(input).finish().unwrap();
            assert_eq!((remaining, parsed.as_str()), (rest, content));
        }

        for input in ["/*", "/* incomplete", "/*nested /* unfinished"] {
            assert!(super::comment(input).finish().is_err());
        }
    }

    #[test]
    fn separator() {
        let (res, _sep) = super::separator("/* comment */").finish().unwrap();
        assert_eq!(res, "");

        let (res, _sep) = super::separator("/* comment1 */ /* comment2 */")
            .finish()
            .unwrap();
        assert_eq!(res, "");

        let (res, _sep) = super::separator(" ").finish().unwrap();
        assert_eq!(res, "");

        assert!(super::separator("").finish().is_err());
    }

    fn tuple_digit(input: &str) -> ParseResult<'_, (char, char)> {
        tuple_((digit, digit)).parse(input)
    }

    #[test]
    fn tuple() {
        let (res, (a, b)) = tuple_digit("1 /* comment */ 2").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(a, '1');
        assert_eq!(b, '2');
    }

    #[test]
    fn tuple_trailing_space() {
        // does not match to trailing space
        let (res, (a, b)) = tuple_digit("1 /* comment */ 2 ").finish().unwrap();
        assert_eq!(res, " ");
        assert_eq!(a, '1');
        assert_eq!(b, '2');
    }

    #[test]
    fn tuple_head_space() {
        // does not match to head space
        assert!(tuple_digit(" 1 /* comment */ 2").finish().is_err());
    }

    fn many0_digit(input: &str) -> ParseResult<'_, Vec<char>> {
        many0_(digit).parse(input)
    }

    #[test]
    fn many0() {
        let (res, digits) =
            many0_digit("1 /* comment */ 2 3").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(digits, &['1', '2', '3']);

        // match to empty
        let (res, digits) = many0_digit("").finish().unwrap();
        assert_eq!(res, "");
        assert!(digits.is_empty());

        let (res, digits) = many1_digit("1").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(digits, &['1']);

        // does not match to trailing space
        let (res, digits) = many0_digit("1 /* comment */ 2 ").finish().unwrap();
        assert_eq!(res, " ");
        assert_eq!(digits, &['1', '2']);

        // does not match to head space
        let (res, digits) = many0_digit(" 1 /* comment */ 2").finish().unwrap();
        assert_eq!(res, " 1 /* comment */ 2"); // match to nothing
        assert!(digits.is_empty());
    }

    fn many1_digit(input: &str) -> ParseResult<'_, Vec<char>> {
        many1_(digit).parse(input)
    }

    #[test]
    fn many1() {
        let (res, digits) =
            many1_digit("1 /* comment */ 2 3").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(digits, &['1', '2', '3']);

        // does not match to empty
        assert!(many1_digit("").finish().is_err());

        let (res, digits) = many1_digit("1").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(digits, &['1']);

        // does not match to trailing space
        let (res, digits) = many1_digit("1 /* comment */ 2 ").finish().unwrap();
        assert_eq!(res, " ");
        assert_eq!(digits, &['1', '2']);

        // does not match to head space
        assert!(many1_digit(" 1 /* comment */ 2").finish().is_err());
    }

    #[test]
    fn ascii_space_scan_matches_nom_and_preserves_utf8_boundaries() {
        for byte in u8::MIN..=u8::MAX {
            for prefix in ["", " ", "\t\r\n", " \t\r\n  \t"] {
                let input = format!("{prefix}{}suffix", char::from(byte));
                let (expected, _) = nom::character::complete::multispace0::<
                    _,
                    VerboseError<_>,
                >(input.as_str())
                .unwrap();
                assert_eq!(super::skip_spaces(&input), expected);
            }
        }
        for input in ["", " \t\r\n", "\u{3000}", "\u{2003}", "\u{feff}"] {
            let (expected, _) = nom::character::complete::multispace0::<
                _,
                VerboseError<_>,
            >(input)
            .unwrap();
            assert_eq!(super::skip_spaces(input), expected);
        }
    }

    #[test]
    fn ignorable() {
        let res = super::ignorable("");
        assert_eq!(res, "");

        let res = super::ignorable(" ");
        assert_eq!(res, "");

        let res = super::ignorable("  ");
        assert_eq!(res, "");

        let res = super::ignorable("/* comment */");
        assert_eq!(res, "");

        let res = super::ignorable("/* comment */ ");
        assert_eq!(res, "");

        let res = super::ignorable(" /* comment */ ");
        assert_eq!(res, "");
    }

    #[test]
    fn ignorable_comments_are_skipped_only_with_an_actual_opener() {
        for (input, rest) in [
            (" /*first*/  /*second*/\n  VALUE", "VALUE"),
            ("/*🦀🌳*/TOKEN", "TOKEN"),
            ("\t\n/* outer /* inner */END", "END"),
            ("\n //not-a-comment", "//not-a-comment"),
            ("\t/foo", "/foo"),
            ("/*unterminated", "/*unterminated"),
            ("  /*unterminated", "/*unterminated"),
            ("/*valid*/  /*unterminated", "/*unterminated"),
        ] {
            let remaining = super::ignorable(input);
            assert_eq!(remaining, rest, "input: {input:?}");
        }
    }

    fn comma_digit(input: &str) -> ParseResult<'_, Vec<char>> {
        comma_separated(digit).parse(input)
    }

    #[test]
    fn repeated_parsers_leave_recoverable_tails_unconsumed_and_propagate_failure()
     {
        assert_eq!(
            comma_digit("1, 2 /*tail*/ ,X"),
            Ok((" /*tail*/ ,X", vec!['1', '2']))
        );
        assert_eq!(
            comma_digit("1, /*unterminated"),
            Ok((", /*unterminated", vec!['1']))
        );
        assert_eq!(many0_digit("1 /*tail*/ X"), Ok((" /*tail*/ X", vec!['1'])));
        let mut parser =
            super::many0_(crate::parser::token::entity_instance_name);
        assert!(matches!(
            parser.parse("#1 #18446744073709551616"),
            Err(nom::Err::Failure(_))
        ));
        let mut empty = super::many0_(super::tag_(""));
        assert!(matches!(empty.parse("X"), Err(nom::Err::Error(_))));
    }

    #[test]
    fn comma() {
        let (res, digits) = comma_digit("1,2").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(digits, &['1', '2']);

        let (res, digits) = comma_digit("1 ,2").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(digits, &['1', '2']);

        let (res, digits) = comma_digit("1, 2").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(digits, &['1', '2']);

        let (res, digits) = comma_digit("1 , 2").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(digits, &['1', '2']);
    }
}
