use crate::{
    ast::{DataSection, EntityInstance, Record, SubSuperRecord},
    parser::{
        combinator::{ParseResult, char_, many0_, opt_, tag_, tuple_},
        exchange::parameter_list,
        token::{entity_instance_name, keyword},
    },
};
use nom::{Parser, branch::alt};

/// `data_section` = `DATA` \[ `(` [`parameter_list`] `)` \] `;`
/// [`entity_instance_list`] `ENDSEC;` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn data_section(input: &str) -> ParseResult<'_, DataSection> {
    tuple_((
        tag_("DATA"),
        opt_(
            tuple_((char_('('), parameter_list, char_(')')))
                .map(|(_open, params, _close)| params),
        ),
        char_(';'),
        entity_instance_list,
        tag_("ENDSEC;"),
    ))
    .map(|(_start, meta, _semicolon, entities, _end)| DataSection {
        meta: meta.unwrap_or_default(),
        entities,
    })
    .parse(input)
}

/// `entity_instance_list` = { [`entity_instance`] } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn entity_instance_list(
    input: &str,
) -> ParseResult<'_, Vec<EntityInstance>> {
    many0_(entity_instance).parse(input)
}

/// `entity_instance` = [`simple_entity_instance`] | [`complex_entity_instance`]
/// .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn entity_instance(input: &str) -> ParseResult<'_, EntityInstance> {
    alt((simple_entity_instance, complex_entity_instance)).parse(input)
}

/// `simple_entity_instance` = [`entity_instance_name`] `=` [`simple_record`]
/// `;` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn simple_entity_instance(input: &str) -> ParseResult<'_, EntityInstance> {
    tuple_((entity_instance_name, char_('='), simple_record, char_(';')))
        .map(|(id, _eq, record, _semicolon)| EntityInstance::Simple {
            id,
            record,
        })
        .parse(input)
}

/// `complex_entity_instance` = [`entity_instance_name`] `=` [`subsuper_record`]
/// `;` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn complex_entity_instance(input: &str) -> ParseResult<'_, EntityInstance> {
    tuple_((
        entity_instance_name,
        char_('='),
        subsuper_record,
        char_(';'),
    ))
    .map(|(id, _eq, subsuper, _semicolon)| EntityInstance::Complex {
        id,
        subsuper,
    })
    .parse(input)
}

/// `simple_record` = [keyword] `(` \[ [`parameter_list`] \] `)` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn simple_record(input: &str) -> ParseResult<'_, Record> {
    tuple_((keyword, char_('('), opt_(parameter_list), char_(')')))
        .map(|(name, _open, parameter, _close)| Record {
            name,
            parameter: parameter.unwrap_or_default().into_iter().collect(),
        })
        .parse(input)
}

/// `simple_record_list` = [`simple_record`] { [`simple_record`] } .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn simple_record_list(input: &str) -> ParseResult<'_, Vec<Record>> {
    many0_(simple_record).parse(input)
}

/// `subsuper_record` = `(` [`simple_record_list`] `)` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn subsuper_record(input: &str) -> ParseResult<'_, SubSuperRecord> {
    tuple_((char_('('), simple_record_list, char_(')')))
        .map(|(_open, records, _close)| SubSuperRecord(records))
        .parse(input)
}

#[cfg(test)]
mod tests {
    use nom::Finish;

    #[test]
    fn simple_recode1() {
        let (res, _record) =
            super::simple_record("A(1, 2.0)").finish().unwrap();
        assert_eq!(res, "");
    }

    #[test]
    fn simple_recode2() {
        let (res, _record) = super::simple_record(
            "LENGTH_MEASURE_WITH_UNIT( LENGTH_MEASURE( 1.00000000000000 ), #359 )",
        )
        .finish()
        .unwrap();
        assert_eq!(res, "");
    }
}
