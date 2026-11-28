//! Exercise the full recursive holder conversion, including missing references.
use std::collections::HashMap;
use step_p21::tables::{IntoOwned, PlaceHolder};
use step_p21_derive::{Holder, as_holder};

#[derive(Default)]
pub struct Table {
    leaf: HashMap<u64, as_holder!(Leaf)>,
    nested: HashMap<u64, as_holder!(Nested)>,
    positional: HashMap<u64, as_holder!(Positional)>,
}

#[derive(Debug, Clone, PartialEq, Holder)]
#[holder(table = Table)]
#[holder(field = leaf)]
pub struct Leaf {
    value: i64,
}

#[derive(Debug, Clone, PartialEq, Holder)]
#[holder(table = Table)]
#[holder(field = nested)]
pub struct Nested {
    #[holder(use_place_holder)]
    entries: Vec<Option<Box<Leaf>>>,
}

#[derive(Debug, Clone, PartialEq, Holder)]
#[holder(table = Table)]
#[holder(field = positional)]
pub struct Positional(#[holder(use_place_holder)] Option<Vec<Box<Leaf>>>);

#[test]
fn nested_holders_resolve_without_losing_container_shape() {
    let table = Table {
        leaf: HashMap::from([(7, LeafHolder { value: 42 })]),
        ..Table::default()
    };
    let reference =
        || Box::new(PlaceHolder::Ref(step_p21::ast::Name::Entity(7)));
    let owned = || Box::new(PlaceHolder::Owned(LeafHolder { value: 3 }));
    let nested = NestedHolder {
        entries: vec![Some(reference()), None, Some(owned())],
    };
    assert_eq!(
        nested.into_owned(&table).unwrap(),
        Nested {
            entries: vec![
                Some(Box::new(Leaf { value: 42 })),
                None,
                Some(Box::new(Leaf { value: 3 }))
            ]
        }
    );
    let positional = PositionalHolder(Some(vec![reference(), owned()]));
    assert_eq!(
        positional.into_owned(&table).unwrap(),
        Positional(Some(vec![
            Box::new(Leaf { value: 42 }),
            Box::new(Leaf { value: 3 })
        ]))
    );
    assert!(
        NestedHolder {
            entries: vec![Some(Box::new(PlaceHolder::Ref(
                step_p21::ast::Name::Entity(99)
            )))]
        }
        .into_owned(&table)
        .is_err()
    );
}
