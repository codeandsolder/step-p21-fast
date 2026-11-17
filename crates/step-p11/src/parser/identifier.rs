use super::{
    basis::simple_id,
    combinator::{EsprParser, ParseResult, alt, remarked},
};

macro_rules! impl_id {
    ($name:ident) => {
        ///
        /// # Errors
        /// Returns a parse error if the input does not satisfy the
        /// corresponding EXPRESS grammar production.
        pub fn $name(input: &str) -> ParseResult<'_, String> {
            remarked(simple_id).parse(input)
        }
    };
}

impl_id!(attribute_id);
impl_id!(constant_id);
impl_id!(entity_id);
impl_id!(enumeration_id);
impl_id!(function_id);
impl_id!(parameter_id);
impl_id!(procedure_id);
impl_id!(rule_label_id);
impl_id!(rule_id);
impl_id!(schema_id);
impl_id!(subtype_constraint_id);
impl_id!(type_label_id);
impl_id!(type_id);
impl_id!(variable_id);

impl_id!(attribute_ref);
impl_id!(constant_ref);
impl_id!(entity_ref);
impl_id!(enumeration_ref);
impl_id!(function_ref);
impl_id!(parameter_ref);
impl_id!(procedure_ref);
impl_id!(rule_label_ref);
impl_id!(rule_ref);
impl_id!(schema_ref);
impl_id!(subtype_constraint_ref);
impl_id!(type_label_ref);
impl_id!(type_ref);
impl_id!(variable_ref);

/// 228 `general_ref` = [`parameter_ref`] | [`variable_ref`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn general_ref(input: &str) -> ParseResult<'_, String> {
    alt((parameter_ref, variable_ref)).parse(input)
}

/// 289 `resource_ref` = [`constant_ref`] | [`entity_ref`] | [`function_ref`] |
/// [`procedure_ref`] | [`type_ref`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn resource_ref(input: &str) -> ParseResult<'_, String> {
    alt((
        constant_ref,
        entity_ref,
        function_ref,
        procedure_ref,
        type_ref,
    ))
    .parse(input)
}

/// 284 `rename_id` = [`constant_id`] | [`entity_id`] | [`function_id`] |
/// [`procedure_id`] | [`type_id`] .
///
/// # Errors
/// Returns a parse error if the input does not satisfy the corresponding
/// EXPRESS grammar production.
pub fn rename_id(input: &str) -> ParseResult<'_, String> {
    alt((constant_id, entity_id, function_id, procedure_id, type_id))
        .parse(input)
}
