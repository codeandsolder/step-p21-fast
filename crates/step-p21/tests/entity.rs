// Test for deserializing Holder structs

use step_p21::{ast::*, parser::exchange, tables::*};

use nom::Finish;
use serde::Deserialize;
use std::str::FromStr;

step_p11_derive::inline_express!(
    r#"
    SCHEMA test_schema;
      ENTITY a;
        x: REAL;
        y: REAL;
      END_ENTITY;

      ENTITY b;
        z: REAL;
        a: a;
      END_ENTITY;
    END_SCHEMA;
    "#
);

use test_schema::*;

const EXAMPLE: &str = r#"
DATA;
  #1 = A(1.0, 2.0);
  #2 = B(3.0, A((4.0, 5.0)));
  #3 = B(6.0, #1);
ENDSEC;
"#;

#[test]
fn deserialize_a_holder() {
    // from Record
    let (residual, p): (_, Record) =
        exchange::simple_record("A(1.0, 2.0)").finish().unwrap();
    assert_eq!(residual, "");
    let a: AHolder = Deserialize::deserialize(&p).unwrap();
    assert_eq!(a, AHolder { x: 1.0, y: 2.0 });

    // from Parameter::Typed
    let (residual, p): (_, Parameter) =
        exchange::parameter("A((1.0, 2.0))").finish().unwrap();
    assert_eq!(residual, "");
    let a: AHolder = Deserialize::deserialize(&p).unwrap();
    assert_eq!(a, AHolder { x: 1.0, y: 2.0 });

    // Test for PlaceHolder<AHolder>
    let (residual, p): (_, Record) =
        exchange::simple_record("A(1.0, 2.0)").finish().unwrap();
    assert_eq!(residual, "");
    let a: PlaceHolder<AHolder> = Deserialize::deserialize(&p).unwrap();
    assert_eq!(a, PlaceHolder::Owned(AHolder { x: 1.0, y: 2.0 }));
}

#[test]
fn deserialize_b_holder_record() {
    // from Record
    let (residual, p): (_, Record) =
        exchange::simple_record("B(1.0, A((2.0, 3.0)))")
            .finish()
            .unwrap();
    assert_eq!(residual, "");
    let b: BHolder = Deserialize::deserialize(&p).unwrap();
    assert_eq!(
        b,
        BHolder {
            z: 1.0,
            a: PlaceHolder::Owned(AHolder { x: 2.0, y: 3.0 })
        }
    );
}

#[test]
fn deserialize_b_holder_record_ref() {
    // from Record with ref
    let (residual, p): (_, Record) =
        exchange::simple_record("B(1.0, #2)").finish().unwrap();
    assert_eq!(residual, "");
    let b: BHolder = Deserialize::deserialize(&p).unwrap();
    assert_eq!(
        b,
        BHolder {
            z: 1.0,
            a: PlaceHolder::Ref(Name::Entity(2))
        }
    );
}

#[test]
fn deserialize_b_holder_parameter() {
    // from Parameter::Typed
    let (residual, p): (_, Parameter) =
        exchange::parameter("B((1.0, A((2.0, 3.0))))")
            .finish()
            .unwrap();
    assert_eq!(residual, "");
    let b: BHolder = Deserialize::deserialize(&p).unwrap();
    assert_eq!(
        b,
        BHolder {
            z: 1.0,
            a: PlaceHolder::Owned(AHolder { x: 2.0, y: 3.0 })
        }
    );
}

#[test]
fn deserialize_b_holder_parameter_ref() {
    // from Parameter::Typed with Ref
    let (residual, p): (_, Parameter) =
        exchange::parameter("B((1.0, #2))").finish().unwrap();
    assert_eq!(residual, "");
    let b: BHolder = Deserialize::deserialize(&p).unwrap();
    assert_eq!(
        b,
        BHolder {
            z: 1.0,
            a: PlaceHolder::Ref(Name::Entity(2))
        }
    );
}

#[test]
fn get_owned_a() {
    let table = Tables::from_str(EXAMPLE).unwrap();
    let a = EntityTable::<AHolder>::get_owned(&table, 1).unwrap();
    assert_eq!(a, A { x: 1.0, y: 2.0 });
}

#[test]
fn get_owned_b2() {
    let table = Tables::from_str(EXAMPLE).unwrap();
    let b = EntityTable::<BHolder>::get_owned(&table, 2).unwrap();
    assert_eq!(
        b,
        B {
            z: 3.0,
            a: A { x: 4.0, y: 5.0 }
        }
    );
}

#[test]
fn get_owned_b3() {
    let table = Tables::from_str(EXAMPLE).unwrap();
    let b = EntityTable::<BHolder>::get_owned(&table, 3).unwrap();
    assert_eq!(
        b,
        B {
            z: 6.0,
            a: A { x: 1.0, y: 2.0 }
        }
    );
}

#[test]
fn duplicate_entity_insertion_preserves_the_original_record() {
    use std::collections::HashMap;

    let first = Record::from_str("A(1, 2)").unwrap();
    let duplicate = Record::from_str("A(99)").unwrap();
    let incompatible = Record::from_str("A('not-an-integer')").unwrap();

    let mut table: HashMap<u64, HashMap<String, Vec<i64>>> = HashMap::new();
    insert_record(&mut table, 7, &first).unwrap();

    for record in [&duplicate, &incompatible] {
        assert!(matches!(
            insert_record(&mut table, 7, record),
            Err(step_p21::error::Error::DuplicatedEntity(7))
        ));
    }
    assert_eq!(table[&7]["A"], [1, 2]);
}

#[test]
fn trailing_step_input_error_reports_only_the_unconsumed_suffix() {
    assert!(matches!(
        Record::from_str("A(1, 2)TRAILING"),
        Err(step_p21::error::Error::ExtraInputRemaining(rest)) if rest == "TRAILING"
    ));
}
