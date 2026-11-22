use crate::ast::{Parameter, Record};
use serde::{
    de::{self, IntoDeserializer},
    forward_to_deserialize_any,
};

impl<'de> de::Deserializer<'de> for &Record {
    type Error = crate::error::Error;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(RecordDeserializer::new(&self.name, &self.parameter))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if name == self.name {
            if let Parameter::List(ref parameters) = self.parameter {
                return visitor.visit_map(RecordStructDeserializer::new(
                    fields, parameters,
                )?);
            }
        }
        Self::deserialize_any(self, visitor)
    }
}

/// Deserializer corresponding to a single-key map like `{ "A": [1.0, 2.0] }`
#[derive(Debug)]
pub struct RecordDeserializer<'record> {
    key: Option<&'record str>,
    value: Option<&'record Parameter>,
    awaiting_value: bool,
}

impl<'de> de::Deserializer<'de> for RecordDeserializer<'_> {
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
        visitor.visit_map(self)
    }
}

impl<'record> RecordDeserializer<'record> {
    #[must_use = "the constructed record deserializer must be used"]
    pub const fn new(key: &'record str, value: &'record Parameter) -> Self {
        RecordDeserializer {
            key: Some(key),
            value: Some(value),
            awaiting_value: false,
        }
    }
}

// Entry point of `visit_map`
impl<'de> de::MapAccess<'de> for RecordDeserializer<'_> {
    type Error = crate::error::Error;

    fn next_key_seed<K>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.awaiting_value {
            return Err(<Self::Error as de::Error>::custom(
                "previous STEP record value was not consumed",
            ));
        }
        let Some(key) = self.key.take() else {
            return Ok(None);
        };
        self.awaiting_value = true;
        let key: de::value::StrDeserializer<Self::Error> =
            key.into_deserializer();
        seed.deserialize(key).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        if !self.awaiting_value {
            return Err(<Self::Error as de::Error>::custom(
                "STEP record value requested without a key",
            ));
        }
        self.awaiting_value = false;
        let value = self.value.take().ok_or_else(|| {
            <Self::Error as de::Error>::custom(
                "STEP record value already consumed",
            )
        })?;
        seed.deserialize(value)
    }
}

/// Deserialize [Record] as a struct
#[derive(Debug)]
pub struct RecordStructDeserializer<'a> {
    cursor: usize,
    fields: &'a [&'static str],
    parameters: &'a [Parameter],
    awaiting_value: bool,
}

impl<'a> RecordStructDeserializer<'a> {
    /// Construct a positional map only when its entire declared shape matches.
    ///
    /// # Errors
    /// Returns an error if the number of parameters differs from the number of
    /// fields.
    pub fn new(
        fields: &'a [&'static str],
        parameters: &'a [Parameter],
    ) -> Result<Self, crate::error::Error> {
        // Validate before handing control to a visitor. It may return
        // successfully without consuming every field.
        if fields.len() != parameters.len() {
            return Err(de::Error::invalid_length(
                parameters.len(),
                &"the exact STEP struct field count",
            ));
        }
        Ok(Self {
            cursor: 0,
            fields,
            parameters,
            awaiting_value: false,
        })
    }
}

impl<'de> de::Deserializer<'de> for RecordStructDeserializer<'_> {
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
        visitor.visit_map(self)
    }
}

impl<'de> de::MapAccess<'de> for RecordStructDeserializer<'_> {
    type Error = crate::error::Error;

    fn next_key_seed<K>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.awaiting_value {
            return Err(<Self::Error as de::Error>::custom(
                "previous STEP struct field value was not consumed",
            ));
        }
        let Some(field) = self.fields.get(self.cursor) else {
            return Ok(None);
        };
        self.awaiting_value = true;
        let key: de::value::StrDeserializer<Self::Error> =
            field.into_deserializer();
        seed.deserialize(key).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        if !self.awaiting_value {
            return Err(<Self::Error as de::Error>::custom(
                "STEP struct field value requested without a key",
            ));
        }
        self.awaiting_value = false;
        // Construction enforces equal lengths, and a pending key implies
        // the cursor addresses an existing field.
        let value = seed.deserialize(&self.parameters[self.cursor])?;
        self.cursor += 1;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::MapAccess;

    #[test]
    fn record_mapping_consumes_each_key_and_value_exactly_once() {
        let param = Parameter::Integer(3);
        let mut access = RecordDeserializer::new("A", &param);
        assert!(access.next_value::<i64>().is_err());
        assert_eq!(access.next_key::<String>().unwrap(), Some("A".to_owned()));
        assert!(access.next_key::<String>().is_err());
        assert_eq!(access.next_value::<i64>().unwrap(), 3);
        assert_eq!(access.next_key::<String>().unwrap(), None);
        assert!(access.next_value::<i64>().is_err());
    }

    #[test]
    fn extra_step_arguments_are_not_silently_ignored() {
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        #[serde(rename = "A")]
        struct Strict {
            x: i64,
            y: i64,
        }

        let extra: Record = "A(1, 2, 3)".parse().unwrap();
        assert!(Strict::deserialize(&extra).is_err());
        let valid: Record = "A(1, 2)".parse().unwrap();
        let decoded = Strict::deserialize(&valid).unwrap();
        assert_eq!((decoded.x, decoded.y), (1, 2));
    }

    #[test]
    fn malformed_struct_shapes_fail_before_the_visitor_runs() {
        use serde::de::{Deserializer, Visitor};
        use std::fmt;

        struct EarlyReturn;

        impl<'de> Visitor<'de> for EarlyReturn {
            type Value = ();

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a STEP struct with two fields")
            }

            fn visit_map<M: MapAccess<'de>>(
                self,
                _map: M,
            ) -> Result<(), M::Error> {
                Ok(())
            }
        }

        for input in ["A(1)", "A(1, 2, 3)"] {
            let record: Record = input.parse().unwrap();
            assert!(
                (&record)
                    .deserialize_struct("A", &["x", "y"], EarlyReturn)
                    .is_err()
            );
        }
        let record: Record = "A(1, 2)".parse().unwrap();
        assert!(
            (&record)
                .deserialize_struct("A", &["x", "y"], EarlyReturn)
                .is_ok()
        );
        assert!(
            RecordStructDeserializer::new(
                &["x", "y"],
                &[Parameter::Integer(1)]
            )
            .is_err()
        );
        assert!(
            RecordStructDeserializer::new(
                &["x"],
                &[Parameter::Integer(1), Parameter::Integer(2)]
            )
            .is_err()
        );
    }
}
