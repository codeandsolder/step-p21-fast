//! An opt-in `#[holder(eq)]` can derive `Eq` for named, tuple, and enum
//! holders.

use std::collections::HashMap;

use step_p21_derive::Holder;

#[derive(Debug, Clone, PartialEq, Eq, Holder)]
#[holder(table = Tables)]
#[holder(eq)]
#[holder(field = named)]
pub struct Named {
    value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Holder)]
#[holder(table = Tables)]
#[holder(eq)]
#[holder(field = tuple)]
pub struct Tuple(i64);

#[derive(Debug, Clone, PartialEq, Eq, Holder)]
#[holder(table = Tables)]
#[holder(eq)]
pub enum Selection {
    Value(i64),
}

#[derive(Debug, Clone, PartialEq, Holder)]
#[holder(table = Tables)]
#[holder(field = float)]
pub struct Floating {
    value: f64,
}

#[derive(Default)]
pub struct Tables {
    pub named: HashMap<u64, NamedHolder>,
    pub tuple: HashMap<u64, TupleHolder>,
    pub float: HashMap<u64, FloatingHolder>,
}

#[test]
fn named_tuple_and_enum_holders_derive_eq() {
    fn requires_eq<T: Eq>() {}

    requires_eq::<NamedHolder>();
    requires_eq::<TupleHolder>();
    requires_eq::<SelectionHolder>();

    assert_eq!(NamedHolder { value: 7 }, NamedHolder { value: 7 });
    assert_eq!(TupleHolder(7), TupleHolder(7));
    assert_eq!(SelectionHolder::Value(7), SelectionHolder::Value(7));
}

#[test]
fn floating_point_holders_remain_partialeq() {
    assert_eq!(FloatingHolder { value: 1.5 }, FloatingHolder { value: 1.5 });
}
