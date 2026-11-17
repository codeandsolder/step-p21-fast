use crate::{
    ast::Parameter,
    parser::{
        combinator::{ParseResult, char_, comma_separated, opt_, tuple_},
        token::{
            enumeration, integer, keyword, real, rhs_occurrence_name, string,
        },
    },
};
use nom::{Parser, branch::alt, combinator::value};

/// list = `(` \[ [parameter] { `,` [parameter] } \] `)` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn list(input: &str) -> ParseResult<'_, Parameter> {
    tuple_((char_('('), opt_(comma_separated(parameter)), char_(')')))
        .map(|(_open, params, _close)| {
            Parameter::List(params.unwrap_or_default())
        })
        .parse(input)
}

/// parameter = [`typed_parameter`] | [`untyped_parameter`] |
/// [`omitted_parameter`] .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn parameter(input: &str) -> ParseResult<'_, Parameter> {
    alt((typed_parameter, untyped_parameter, omitted_parameter)).parse(input)
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

/// `untyped_parameter` = `$` | [integer] | [real] | [string] |
/// [`rhs_occurrence_name`] | [enumeration] | binary | [list] .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn untyped_parameter(input: &str) -> ParseResult<'_, Parameter> {
    alt((
        char_('$').map(|_| Parameter::NotProvided),
        real.map(Parameter::Real),
        integer.map(Parameter::Integer),
        string.map(Parameter::String),
        rhs_occurrence_name.map(Parameter::Ref),
        enumeration.map(Parameter::Enumeration),
        // FIXME binary
        list,
    ))
    .parse(input)
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
