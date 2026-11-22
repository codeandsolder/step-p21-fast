use crate::{
    ast::Parameter,
    parser::{
        combinator::{
            ParseResult, char_, comma_separated, parenthesized_list, tuple_,
        },
        token::{enumeration, keyword, number, rhs_occurrence_name, string},
    },
};
use nom::{
    Parser,
    combinator::value,
    error::{ErrorKind, ParseError},
};

/// list = `(` \[ [parameter] { `,` [parameter] } \] `)` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn list(input: &str) -> ParseResult<'_, Parameter> {
    parenthesized_list(parameter)
        .map(Parameter::List)
        .parse(input)
}

/// parameter = [`typed_parameter`] | [`untyped_parameter`] |
/// [`omitted_parameter`] .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn parameter(input: &str) -> ParseResult<'_, Parameter> {
    parameter_dispatch::<true>(input)
}

/// `typed_parameter` = [keyword] `(` [parameter] `)` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn typed_parameter(input: &str) -> ParseResult<'_, Parameter> {
    tuple_((keyword, char_('('), parameter, char_(')')))
        .map(|(name, _open, ty, _close)| Parameter::Typed {
            keyword: name,
            parameter: Box::new(ty),
        })
        .parse(input)
}

/// `untyped_parameter` = `$` | [`integer`](crate::parser::token::integer) |
/// [`real`](crate::parser::token::real) | [string] |
/// [`rhs_occurrence_name`] | [enumeration] | binary | [list] .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn untyped_parameter(input: &str) -> ParseResult<'_, Parameter> {
    parameter_dispatch::<false>(input)
}

/// Both grammar productions use the same token classification. Only the
/// full parameter grammar admits typed or omitted values.
fn parameter_dispatch<const FULL: bool>(
    input: &str,
) -> ParseResult<'_, Parameter> {
    match input.as_bytes() {
        [b'A'..=b'Z' | b'_' | b'!', ..] if FULL => typed_parameter(input),
        [b'*', ..] if FULL => Ok((&input[1..], Parameter::Omitted)),
        [b'$', ..] => Ok((&input[1..], Parameter::NotProvided)),
        [b'0'..=b'9' | b'+' | b'-' | b' ' | b'\t' | b'\r' | b'\n', ..] => {
            number(input)
        }
        [b'\'', ..] => {
            string(input).map(|(rest, value)| (rest, Parameter::String(value)))
        }
        [b'#' | b'@', ..] => rhs_occurrence_name(input)
            .map(|(rest, value)| (rest, Parameter::Ref(value))),
        [b'.', ..] => enumeration(input)
            .map(|(rest, value)| (rest, Parameter::Enumeration(value))),
        [b'(', ..] => list(input),
        // Binary literals are not yet supported.
        _ => Err(nom::Err::Error(ParseError::from_error_kind(
            input,
            ErrorKind::Alt,
        ))),
    }
}

/// `omitted_parameter` = `*` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn omitted_parameter(input: &str) -> ParseResult<'_, Parameter> {
    value(Parameter::Omitted, char_('*')).parse(input)
}

/// `parameter_list` = [parameter] { `,` [parameter] } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn parameter_list(input: &str) -> ParseResult<'_, Vec<Parameter>> {
    comma_separated(parameter).parse(input)
}

#[cfg(test)]
mod tests {
    use crate::ast::Parameter;
    use nom::Finish;

    // The former ordered grammar alternatives are an independent oracle
    // for choosing a syntax class, including recovery after partial input.
    fn reference_parameter(input: &str) -> super::ParseResult<'_, Parameter> {
        use nom::{Parser, branch::alt};

        alt((
            super::typed_parameter,
            reference_untyped,
            super::omitted_parameter,
        ))
        .parse(input)
    }

    fn reference_untyped(input: &str) -> super::ParseResult<'_, Parameter> {
        use crate::parser::token::{
            enumeration, integer, real, rhs_occurrence_name, string,
        };
        use nom::{Parser, branch::alt, combinator::value};

        alt((
            value(Parameter::NotProvided, super::char_('$')),
            real.map(Parameter::Real),
            integer.map(Parameter::Integer),
            string.map(Parameter::String),
            rhs_occurrence_name.map(Parameter::Ref),
            enumeration.map(Parameter::Enumeration),
            super::list,
        ))
        .parse(input)
    }

    #[test]
    fn parenthesized_lists_accept_empty_comments_and_reject_invalid_tails() {
        for input in ["()", "( \t\r\n)", "(/*empty*/ )"] {
            assert_eq!(
                super::list(input),
                Ok(("", Parameter::List(Vec::new())))
            );
        }
        assert_eq!(
            super::list("(/*before*/ 1, #2, () /*after*/)tail"),
            Ok((
                "tail",
                Parameter::List(vec![
                    Parameter::Integer(1),
                    Parameter::Ref(crate::ast::Name::Entity(2)),
                    Parameter::List(Vec::new()),
                ])
            ))
        );
        assert_eq!(
            super::list("( )missing"),
            Ok(("missing", Parameter::List(Vec::new())))
        );
        for input in ["(,)", "(1,)", "(/*invalid", "(1 2)", "("] {
            assert!(super::list(input).is_err(), "{input:?}");
        }
        assert!(matches!(
            super::list("(#18446744073709551616)"),
            Err(nom::Err::Failure(_))
        ));
    }

    #[test]
    fn dispatch_matches_ordered_alternatives_for_tokens_and_residuals() {
        for token in [
            "",
            "a",
            "é",
            "\"BINARY\"",
            "!",
            "A",
            "_",
            "A(1)",
            "!A((1,$,*))",
            "A /* comment */ (1.2)",
            "A()",
            "$",
            "*",
            ".T.",
            ".FALSE.",
            ".",
            ".1",
            "#",
            "@",
            "#0",
            "@001",
            "#UPPER",
            "@_VALUE",
            "#é",
            "#-1",
            "#18446744073709551615",
            "#18446744073709551616",
            "@18446744073709551616",
            "'é''abc'",
            "'''",
            "()",
            "(1,2,())",
            "(1,)",
            "((1)",
            "1",
            "+1",
            "-0",
            " 1",
            "\t-1",
            " -1",
            "- 1",
            "+\r\n2",
            "9223372036854775808",
            "-9223372036854775808",
            "1.",
            "-0.0",
            "1.25E  -  03",
            "1.2E",
            "1.2E+",
            "1.2E9223372036854775808",
            "1.2E309",
            "-1.2E-400",
            "1.2e3",
            "1E3",
            "-",
            "+",
            "  ",
            " $",
            "\n#1",
        ] {
            for suffix in ["", ",", "/*next*/", "é", " ", "0"] {
                let input = format!("{token}{suffix}");
                for (actual, expected) in [
                    (super::parameter(&input), reference_parameter(&input)),
                    (
                        super::untyped_parameter(&input),
                        reference_untyped(&input),
                    ),
                ] {
                    match (actual, expected) {
                        (
                            Ok((rest, Parameter::Real(actual))),
                            Ok((expected_rest, Parameter::Real(expected))),
                        ) => {
                            assert_eq!(rest, expected_rest, "{input:?}");
                            assert_eq!(
                                actual.to_bits(),
                                expected.to_bits(),
                                "{input:?}"
                            );
                        }
                        (Ok(actual), Ok(expected)) => {
                            assert_eq!(actual, expected, "{input:?}")
                        }
                        (Err(nom::Err::Error(_)), Err(nom::Err::Error(_)))
                        | (
                            Err(nom::Err::Failure(_)),
                            Err(nom::Err::Failure(_)),
                        ) => {}
                        (actual, expected) => {
                            assert_eq!(actual, expected, "{input:?}")
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn untyped_parameter() {
        let (res, record) = super::untyped_parameter("2").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(record, Parameter::integer(2));

        let (res, record) = super::untyped_parameter("2.0").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(record, Parameter::real(2.0));
    }

    #[test]
    fn parameter_list() {
        let (res, record) =
            super::untyped_parameter("(1, 2, 3)").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(
            record,
            Parameter::List(vec![
                Parameter::Integer(1),
                Parameter::Integer(2),
                Parameter::Integer(3),
            ])
        );
    }

    #[test]
    fn empty_parameter_list() {
        let (res, record) = super::untyped_parameter("()").finish().unwrap();
        assert_eq!(res, "");
        assert_eq!(record, Parameter::List(vec![]));
    }
}
