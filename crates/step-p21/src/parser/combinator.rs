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
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{char, multispace0, multispace1},
    combinator::{opt, value},
    multi::{many0, many1},
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
    move |input| {
        let (input, c) = nom::character::complete::char(c).parse(input)?;
        Ok((input, c))
    }
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
    let (remaining, text) = take_until("*/").parse(remaining)?;
    let (remaining, _) = tag("*/")(remaining)?;
    Ok((remaining, text))
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
    let comment = many1((multispace0, comment_slice, multispace0)).map(|_| ());
    alt((comment, value((), multispace1))).parse(input)
}

pub fn many0_<'a, O>(
    f: impl ExchangeParser<'a, O>,
) -> impl ExchangeParser<'a, Vec<O>> {
    move |input| {
        let (input, first) = match opt(f.clone()).parse(input)? {
            (input, Some(first)) => (input, first),
            (input, None) => return Ok((input, Vec::new())),
        };
        let (input, tail) =
            many0((ignorable, f.clone()).map(|(_sep, v)| v)).parse(input)?;
        Ok((input, std::iter::once(first).chain(tail).collect()))
    }
}

pub fn many1_<'a, O>(
    f: impl ExchangeParser<'a, O>,
) -> impl ExchangeParser<'a, Vec<O>> {
    move |input| {
        (f.clone(), many0((ignorable, f.clone())))
            .map(|(first, tail)| {
                let first = vec![first];
                let tail = tail.into_iter().map(|(_sep, val)| val);
                first.into_iter().chain(tail).collect()
            })
            .parse(input)
    }
}

///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn ignorable(mut input: &str) -> ParseResult<'_, ()> {
    // Speculatively parsing a comment at every token boundary allocates a
    // VerboseError even when the input contains only ordinary whitespace.
    // Comments are unambiguous at the byte level, so only invoke their parser
    // after the slash-star opener is present.
    loop {
        let (after_spaces, _) = multispace0(input)?;
        input = after_spaces;
        if !input.starts_with("/*") {
            return Ok((input, ()));
        }
        match comment_slice(input) {
            Ok((after_comment, _)) => input = after_comment,
            // Preserve the old backtracking behavior for an incomplete
            // comment opener: ignore the preceding whitespace but let the
            // following token parser decide how to report the invalid input.
            Err(nom::Err::Error(_)) => return Ok((input, ())),
            Err(error) => return Err(error),
        }
    }
}

pub fn separated<'a, O>(
    c: char,
    f: impl ExchangeParser<'a, O>,
) -> impl ExchangeParser<'a, Vec<O>> {
    move |input| {
        (
            f.clone(),
            many0(
                (ignorable, char(c), ignorable, f.clone())
                    .map(|(_sep1, _char, _sep2, value)| value),
            ),
        )
            .map(|(first, mut tails)| {
                let mut values = vec![first];
                values.append(&mut tails);
                values
            })
            .parse(input)
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

/// Build nested native nom tuples, separated by ignorable input.
macro_rules! tuple_gen {
    ($head:ident, $($tail:ident),*) => {
        ($head.clone(), ignorable, tuple_gen!($($tail),*))
    };
    ($head:ident) => {
        $head.clone()
    };
}

/// Expand `match_gen!(o1, o2, o3)` to `(o1, _, (o2, _, o3))`
macro_rules! match_gen {
    ($head:ident, $($tail:ident),*) => {
        ($head, _, match_gen!($($tail),*))
    };
    ($head:ident) => {
        $head
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
                tuple_gen!($($f),*)
                    .map(|match_gen!($($o),*)| ($($o),*))
                    .parse(input)
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
    fn ignorable() {
        let (res, _) = super::ignorable("").finish().unwrap();
        assert_eq!(res, "");

        let (res, _) = super::ignorable(" ").finish().unwrap();
        assert_eq!(res, "");

        let (res, _) = super::ignorable("  ").finish().unwrap();
        assert_eq!(res, "");

        let (res, _) = super::ignorable("/* comment */").finish().unwrap();
        assert_eq!(res, "");

        let (res, _) = super::ignorable("/* comment */ ").finish().unwrap();
        assert_eq!(res, "");

        let (res, _) = super::ignorable(" /* comment */ ").finish().unwrap();
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
            let (remaining, ()) = super::ignorable(input).unwrap();
            assert_eq!(remaining, rest, "input: {input:?}");
        }
    }

    fn comma_digit(input: &str) -> ParseResult<'_, Vec<char>> {
        comma_separated(digit).parse(input)
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
