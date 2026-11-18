use super::super::combinator::{
    EsprParser, ParseResult, alt, char, is_not, opt, tag, tuple, value,
};
use crate::ast::{SimpleType, WidthSpec};

/// 307 `simple_types` = [`binary_type`] | [`boolean_type`] | [`integer_type`] |
/// [`logical_type`] | [`number_type`] | [`real_type`] | [`string_type`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn simple_types(input: &str) -> ParseResult<'_, SimpleType> {
    alt((
        number_type,
        real_type,
        integer_type,
        logical_type,
        boolean_type,
        string_type,
        binary_type,
    ))
    .parse(input)
}

/// 261 `number_type` = NUMBER .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn number_type(input: &str) -> ParseResult<'_, SimpleType> {
    value(SimpleType::Number, tag("NUMBER")).parse(input)
}

/// 278 `real_type` = REAL \[ `(` `precision_spec` `)` \] .
///
/// 268 `precision_spec` = `numeric_expression` .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn real_type(input: &str) -> ParseResult<'_, SimpleType> {
    // FIXME precision_spec is not supported
    value(SimpleType::Real, tag("REAL")).parse(input)
}

/// 241 `integer_type` = INTEGER .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn integer_type(input: &str) -> ParseResult<'_, SimpleType> {
    value(SimpleType::Integer, tag("INTEGER")).parse(input)
}

/// 256 `logical_type` = LOGICAL .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn logical_type(input: &str) -> ParseResult<'_, SimpleType> {
    value(SimpleType::Logical, tag("LOGICAL")).parse(input)
}

/// 182 `boolean_type` = BOOLEAN .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn boolean_type(input: &str) -> ParseResult<'_, SimpleType> {
    value(SimpleType::Boolen, tag("BOOLEAN")).parse(input)
}

/// 311 `string_type` = STRING \[ [`width_spec`] \] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn string_type(input: &str) -> ParseResult<'_, SimpleType> {
    tuple((tag("STRING"), opt(width_spec)))
        .map(|(_, width_spec)| SimpleType::String_ { width_spec })
        .parse(input)
}

/// 181 `binary_type` = BINARY \[ [`width_spec`] \] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn binary_type(input: &str) -> ParseResult<'_, SimpleType> {
    tuple((tag("BINARY"), opt(width_spec)))
        .map(|(_, width_spec)| SimpleType::Binary { width_spec })
        .parse(input)
}

/// 341 `width_spec` = `(` width `)` \[ FIXED \] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn width_spec(input: &str) -> ParseResult<'_, WidthSpec> {
    // FIXME Should use `numeric_expression` parser
    let (rest, ((_, width, _, fixed), remarks)) =
        tuple((char('('), is_not(")"), char(')'), opt(tag("FIXED"))))
            .parse(input)?;
    let width = width.parse::<usize>().map_err(|_| {
        nom::Err::Failure(nom::error::VerboseError {
            errors: vec![(
                rest,
                nom::error::VerboseErrorKind::Context("width-spec-overflow"),
            )],
        })
    })?;
    Ok((
        rest,
        (
            WidthSpec {
                width,
                fixed: fixed.is_some(),
            },
            remarks,
        ),
    ))
}

#[cfg(test)]
mod tests {

    #[test]
    fn rejects_oversized_width_spec_without_panicking() {
        assert!(super::width_spec("(18446744073709551616)").is_err());
    }

    use crate::ast::{SimpleType, WidthSpec};
    use nom::Finish;

    #[test]
    fn string() {
        let (res, (string, _remarks)) =
            super::string_type("STRING").finish().unwrap();
        assert_eq!(string, SimpleType::String_ { width_spec: None });
        assert_eq!(res, "");

        let (res, (string, _remarks)) =
            super::string_type("STRING (10)").finish().unwrap();
        assert_eq!(
            string,
            SimpleType::String_ {
                width_spec: Some(WidthSpec {
                    width: 10,
                    fixed: false,
                })
            }
        );
        assert_eq!(res, "");

        let (res, (string, _remarks)) =
            super::string_type("STRING (10) FIXED").finish().unwrap();
        assert_eq!(
            string,
            SimpleType::String_ {
                width_spec: Some(WidthSpec {
                    width: 10,
                    fixed: true,
                }),
            }
        );
        assert_eq!(res, "");
    }

    #[test]
    fn binary() {
        let (res, (binary, _remarks)) =
            super::binary_type("BINARY").finish().unwrap();
        assert_eq!(binary, SimpleType::Binary { width_spec: None });
        assert_eq!(res, "");

        let (res, (binary, _remarks)) =
            super::binary_type("BINARY (10)").finish().unwrap();
        assert_eq!(
            binary,
            SimpleType::Binary {
                width_spec: Some(WidthSpec {
                    width: 10,
                    fixed: false
                })
            }
        );
        assert_eq!(res, "");

        let (res, (binary, _remarks)) =
            super::binary_type("BINARY (10) FIXED").finish().unwrap();
        assert_eq!(
            binary,
            SimpleType::Binary {
                width_spec: Some(WidthSpec {
                    width: 10,
                    fixed: true
                })
            }
        );
        assert_eq!(res, "");
    }
}
