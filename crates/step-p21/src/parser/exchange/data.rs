use crate::{
    ast::{DataSection, EntityInstance, Parameter, Record, SubSuperRecord},
    parser::{
        combinator::{
            ParseResult, char_, many0_, opt_, skip_ignorable, tag_, tuple_,
        },
        exchange::{parameter::parenthesized_parameters, parameter_list},
        token::{entity_instance_name, keyword},
    },
};
use nom::Parser;

/// `data_section` = `DATA` \[ `(` [`parameter_list`] `)` \] `;`
/// [`entity_instance_list`] `ENDSEC;` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn data_section(input: &str) -> ParseResult<'_, DataSection> {
    tuple_((
        tag_("DATA"),
        opt_(tuple_((char_('('), parameter_list, char_(')')))),
        char_(';'),
        entity_instance_list,
        tag_("ENDSEC;"),
    ))
    .map(|(_start, meta, _semicolon, entities, _end)| DataSection {
        meta: meta.map_or_else(Vec::new, |(_open, params, _close)| params),
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
    let (rest, id) = entity_instance_prefix(input)?;
    if rest.starts_with('(') {
        complex_entity_body(rest, id)
    } else {
        simple_entity_body(rest, id)
    }
}

fn entity_instance_prefix(input: &str) -> ParseResult<'_, u64> {
    let (rest, (id, _eq)) =
        tuple_((entity_instance_name, char_('='))).parse(input)?;
    let rest = skip_ignorable(rest);
    Ok((rest, id))
}

/// `simple_entity_instance` = [`entity_instance_name`] `=` [`simple_record`]
/// `;` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn simple_entity_instance(input: &str) -> ParseResult<'_, EntityInstance> {
    let (rest, id) = entity_instance_prefix(input)?;
    simple_entity_body(rest, id)
}

fn simple_entity_body(input: &str, id: u64) -> ParseResult<'_, EntityInstance> {
    let (rest, (record, _semicolon)) =
        tuple_((simple_record, char_(';'))).parse(input)?;
    Ok((rest, EntityInstance::Simple { id, record }))
}

/// `complex_entity_instance` = [`entity_instance_name`] `=` [`subsuper_record`]
/// `;` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn complex_entity_instance(input: &str) -> ParseResult<'_, EntityInstance> {
    let (rest, id) = entity_instance_prefix(input)?;
    complex_entity_body(rest, id)
}

fn complex_entity_body(
    input: &str,
    id: u64,
) -> ParseResult<'_, EntityInstance> {
    let (rest, (subsuper, _semicolon)) =
        tuple_((subsuper_record, char_(';'))).parse(input)?;
    Ok((rest, EntityInstance::Complex { id, subsuper }))
}

/// `simple_record` = [keyword] `(` \[ [`parameter_list`] \] `)` .
///
/// # Errors
/// Returns a parse error when the input does not satisfy the corresponding STEP
/// grammar production.
pub fn simple_record(input: &str) -> ParseResult<'_, Record> {
    tuple_((keyword, parenthesized_parameters))
        .map(|(name, parameters)| Record {
            name,
            parameter: Parameter::List(parameters),
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
    fn instance_dispatch_matches_ordered_alternatives_and_residuals() {
        use nom::{Parser, branch::alt};

        for record in [
            "A(1)",
            "!CUSTOM_1(/*empty*/)",
            "(A(1) B(2.0))",
            "(A()/*comment*/B(#2))",
            "()",
            "A(",
            "B(,)",
            "(A(1),B(2))",
            "(A(#18446744073709551616))",
            "A(#18446744073709551616)",
            "bad",
            "",
        ] {
            for id in [
                "#0001",
                "#18446744073709551615",
                "#18446744073709551616",
                "@1",
                "#A",
            ] {
                let input = format!("{id} /*left*/ = /*right*/ {record} ;rest");
                let expected = alt((
                    super::simple_entity_instance,
                    super::complex_entity_instance,
                ))
                .parse(&input);
                let actual = super::entity_instance(&input);
                match (actual, expected) {
                    (Err(nom::Err::Error(_)), Err(nom::Err::Error(_)))
                    | (Err(nom::Err::Failure(_)), Err(nom::Err::Failure(_))) => {
                    }
                    (actual, expected) => {
                        assert_eq!(actual, expected, "{input:?}")
                    }
                }
            }
        }
    }

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
