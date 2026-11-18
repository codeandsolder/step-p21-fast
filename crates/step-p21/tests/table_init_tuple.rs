//! Check the shared `TableInit` generator for positional maps and holder
//! variants.

use std::collections::HashMap;

use step_p21::{ast::Record, error::Error, tables::EntityTable};
use step_p21_derive::{Holder, TableInit};

#[derive(Debug, Clone, PartialEq, Holder)]
#[holder(table = PositionalTable)]
#[holder(field = 0)]
#[holder(generate_deserialize)]
pub struct Alpha {
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Holder)]
#[holder(table = PositionalTable)]
#[holder(field = 1)]
#[holder(generate_deserialize)]
pub struct Beta {
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Holder)]
#[holder(table = PositionalTable)]
#[holder(field = 2)]
#[holder(generate_deserialize)]
pub struct Measurement(pub i64, pub f64);

#[derive(Default, TableInit)]
pub struct PositionalTable(
    pub HashMap<u64, AlphaHolder>,
    pub HashMap<u64, BetaHolder>,
    pub HashMap<u64, MeasurementHolder>,
);

#[test]
fn positional_table_dispatch_and_reference_resolution() {
    let table: PositionalTable =
        "DATA; #1=ALPHA(10); #2=BETA(2.5); #3=MEASUREMENT(7,3.5); ENDSEC;"
            .parse()
            .expect("valid data section");
    assert_eq!(table.0.get(&1), Some(&AlphaHolder { value: 10 }));
    assert_eq!(table.1.get(&2), Some(&BetaHolder { value: 2.5 }));
    assert_eq!(table.2.get(&3), Some(&MeasurementHolder(7, 3.5)));
    assert_eq!(
        EntityTable::<AlphaHolder>::get_owned(&table, 1).expect("entity 1"),
        Alpha { value: 10 },
    );
    assert_eq!(
        EntityTable::<MeasurementHolder>::get_owned(&table, 3)
            .expect("entity 3"),
        Measurement(7, 3.5),
    );
}

#[test]
fn positional_table_reports_unknown_entities_without_panicking() {
    let result = "DATA; #1=GAMMA(1); ENDSEC;".parse::<PositionalTable>();
    assert!(matches!(result, Err(Error::UnknownEntityName { .. })));
}

#[test]
fn positional_table_rejects_complex_entities_with_an_error() {
    let result =
        "DATA; #1=(ALPHA(1) BETA(2.5)); ENDSEC;".parse::<PositionalTable>();
    assert!(matches!(result, Err(Error::DeserializeFailed(_))));
}

#[test]
fn named_and_tuple_visitors_report_truncated_sequences() {
    let named: Record = "ALPHA()".parse().expect("valid empty record");
    let tuple: Record =
        "MEASUREMENT(1)".parse().expect("valid truncated record");
    let named_error =
        <AlphaHolder as step_p21::serde::Deserialize>::deserialize(&named);
    let tuple_error =
        <MeasurementHolder as step_p21::serde::Deserialize>::deserialize(
            &tuple,
        );
    assert!(named_error.is_err());
    assert!(tuple_error.is_err());
}
