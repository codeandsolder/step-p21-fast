use super::super::combinator::{EsprParser, ParseResult, alt, tag, value};
use crate::ast::{
    BinaryOperator, IntervalOperator, RelationOperator, UnaryOperator,
};

/// 282 `rel_op` = `<` | `>` | `<=` | `>=` | `<>` | `=` | `:<>:` | `:=:` .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn rel_op(input: &str) -> ParseResult<'_, RelationOperator> {
    alt((
        value(RelationOperator::InstanceEqual, tag(":=:")),
        value(RelationOperator::InstanceNotEqual, tag(":<>:")),
        value(RelationOperator::Leq, tag("<=")),
        value(RelationOperator::Geq, tag(">=")),
        value(RelationOperator::Equal, tag("=")),
        value(RelationOperator::NotEqual, tag("<>")),
        value(RelationOperator::Lt, tag("<")),
        value(RelationOperator::Gt, tag(">")),
    ))
    .parse(input)
}

/// 283 `rel_op_extended` = [`rel_op`] | `IN` | `LIKE` .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn rel_op_extended(input: &str) -> ParseResult<'_, RelationOperator> {
    alt((
        rel_op,
        alt((
            value(RelationOperator::In, tag("IN")),
            value(RelationOperator::Like, tag("LIKE")),
        )),
    ))
    .parse(input)
}

/// 331 `unary_op` = `+` | `-` | `NOT` .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn unary_op(input: &str) -> ParseResult<'_, UnaryOperator> {
    alt((
        value(UnaryOperator::Plus, tag("+")),
        value(UnaryOperator::Minus, tag("-")),
        value(UnaryOperator::Not, tag("NOT")),
    ))
    .parse(input)
}

/// 168 `add_like_op` = `+` | `-` | `OR` | `XOR` .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn add_like_op(input: &str) -> ParseResult<'_, BinaryOperator> {
    use BinaryOperator::{Add, Or, Sub, Xor};
    alt((
        value(Add, tag("+")),
        value(Sub, tag("-")),
        value(Or, tag("OR")),
        value(Xor, tag("XOR")),
    ))
    .parse(input)
}

/// 257 `multiplication_like_op` = `*` | `/` | `DIV` | `MOD` | `AND` | `||` .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn multiplication_like_op(input: &str) -> ParseResult<'_, BinaryOperator> {
    alt((
        value(BinaryOperator::Mul, tag("*")),
        value(BinaryOperator::RealDiv, tag("/")),
        value(BinaryOperator::IntegerDiv, tag("DIV")),
        value(BinaryOperator::Mod, tag("MOD")),
        value(BinaryOperator::And, tag("AND")),
        value(BinaryOperator::ComplexEntityInstanceConstruction, tag("||")),
    ))
    .parse(input)
}

/// 999 `power_op` = `**`
///
/// Additional trivial rule for managing operators uniformly
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn power_op(input: &str) -> ParseResult<'_, BinaryOperator> {
    value(BinaryOperator::Power, tag("**")).parse(input)
}

/// 247 `interval_op` = `<` | `<=` .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn interval_op(input: &str) -> ParseResult<'_, IntervalOperator> {
    alt((
        value(IntervalOperator::LessThanEqual, tag("<=")),
        value(IntervalOperator::LessThan, tag("<")),
    ))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use nom::Finish;

    #[test]
    fn rel_op() {
        let (res, (_op, _remarks)) = super::rel_op("<=").finish().unwrap();
        assert_eq!(res, "");
    }
}
