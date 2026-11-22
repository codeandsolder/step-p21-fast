use crate::ast::{Parameter, Record, SubSuperRecord};
use serde::{
    de::{self, IntoDeserializer},
    forward_to_deserialize_any,
};

impl<'de> de::Deserializer<'de> for &SubSuperRecord {
    type Error = crate::error::Error;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        struct tuple_struct map enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(SubSuperRecordDeserializer::new(self.0.as_slice()))
    }
}

#[derive(Debug)]
pub struct SubSuperRecordDeserializer<'record> {
    remaining: &'record [Record],
    pending_value: Option<&'record Parameter>,
}

impl<'record> SubSuperRecordDeserializer<'record> {
    #[must_use = "the constructed complex entity deserializer must be used"]
    pub const fn new(records: &'record [Record]) -> Self {
        Self {
            remaining: records,
            pending_value: None,
        }
    }
}

// Entry point of `visit_map`
impl<'de> de::MapAccess<'de> for SubSuperRecordDeserializer<'_> {
    type Error = crate::error::Error;

    fn next_key_seed<K>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.pending_value.is_some() {
            return Err(<Self::Error as de::Error>::custom(
                "previous complex STEP record value was not consumed",
            ));
        }
        let Some((record, remaining)) = self.remaining.split_last() else {
            return Ok(None);
        };
        // Preserve the original reverse record iteration without allocating
        // parallel keyword and parameter vectors for every deserialization.
        self.remaining = remaining;
        self.pending_value = Some(&record.parameter);
        let key: de::value::StrDeserializer<Self::Error> =
            record.name.as_str().into_deserializer();
        seed.deserialize(key).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let value = self.pending_value.take().ok_or_else(|| {
            <Self::Error as de::Error>::custom(
                "complex STEP record value requested without a key",
            )
        })?;
        seed.deserialize(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::MapAccess;

    #[test]
    fn complex_records_deserialize_in_existing_reverse_order_without_allocations()
     {
        let records = [
            Record {
                name: "A".to_owned(),
                parameter: Parameter::List(vec![Parameter::Integer(1)]),
            },
            Record {
                name: "B".to_owned(),
                parameter: Parameter::List(vec![Parameter::Integer(2)]),
            },
        ];
        let mut access = SubSuperRecordDeserializer::new(&records);
        assert_eq!(access.next_key::<String>().unwrap(), Some("B".to_owned()));
        assert_eq!(access.next_value::<Vec<i64>>().unwrap(), vec![2]);
        assert_eq!(access.next_key::<String>().unwrap(), Some("A".to_owned()));
        assert_eq!(access.next_value::<Vec<i64>>().unwrap(), vec![1]);
        assert_eq!(access.next_key::<String>().unwrap(), None);
    }

    #[test]
    fn complex_records_reject_missing_key_or_unconsumed_value() {
        let records = [Record {
            name: "A".to_owned(),
            parameter: Parameter::Integer(5),
        }];
        let mut access = SubSuperRecordDeserializer::new(&records);
        assert!(access.next_value::<i64>().is_err());
        assert_eq!(access.next_key::<String>().unwrap(), Some("A".to_owned()));
        assert!(access.next_key::<String>().is_err());
        assert_eq!(access.next_value::<i64>().unwrap(), 5);
        assert_eq!(access.next_key::<String>().unwrap(), None);
    }
}
