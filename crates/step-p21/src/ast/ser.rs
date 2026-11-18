use crate::{
    ast::{Parameter, Record},
    error::{Error, Result},
};
use inflector::Inflector;
use serde::ser;
use std::convert::TryFrom;

/// Serialize struct into STEP [Record]
///
/// # Errors
/// Returns an error when the provided value cannot be serialized as a STEP
/// record.
pub fn to_record(obj: &impl ser::Serialize) -> Result<Record> {
    let mut ser = RecordSerializer::default();
    obj.serialize(&mut ser)?;
    if !ser.stack.is_empty() {
        return Err(Error::UnbalancedRecordStack);
    }
    if ser.name.is_empty() {
        return Err(Error::UnsupportedSerialization(
            "top-level STEP records must be structs",
        ));
    }
    Ok(Record {
        name: ser.name,
        parameter: ser.parameters.iter().collect(),
    })
}

#[derive(Default, Debug)]
struct RecordSerializer {
    name: String,
    parameters: Vec<Parameter>,
    // For supporting nested record e.g. `B(3.0, A((1.0, 2.0)))`
    stack: Vec<(String, Vec<Parameter>)>,
}

impl ser::Serializer for &mut RecordSerializer {
    type Error = Error;
    type Ok = ();
    type SerializeMap = Self;
    type SerializeSeq = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        if v {
            self.parameters
                .push(Parameter::Enumeration("TRUE".to_string()));
        } else {
            self.parameters
                .push(Parameter::Enumeration("FALSE".to_string()));
        }
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.parameters.push(Parameter::Integer(v));
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.serialize_i64(
            i64::try_from(v).map_err(|_| Error::IntegerOutOfRange(v))?,
        )
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.parameters.push(Parameter::Real(v));
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.parameters.push(Parameter::String(v.to_string()));
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::UnsupportedSerialization("byte strings"))
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.parameters.push(Parameter::NotProvided);
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.parameters
            .push(Parameter::Enumeration(variant.to_screaming_snake_case()));
        Ok(())
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        Err(Error::UnsupportedSerialization("newtype enum variants"))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::UnsupportedSerialization("tuple enum variants"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::UnsupportedSerialization("maps"))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        // Rust's serde names are commonly PascalCase; Part 21 records use
        // uppercase keywords with underscores, also for nested typed values.
        let name = name.to_screaming_snake_case();
        if self.name.is_empty() {
            self.name = name;
        } else {
            // Entering sub struct e.g.
            //
            // ```
            // B(3.0, A((1.0, 2.0))) ^ here
            // ```
            //
            // Put current parameters (`3.0` as above) onto the top of stack,
            // and start serializing `A((1.0, 2.0))`.
            // This stack will be popped in SerializeStruct::end()
            //
            let current_name = std::mem::replace(&mut self.name, name);
            let current_params = std::mem::take(&mut self.parameters);
            self.stack.push((current_name, current_params));
        }
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::UnsupportedSerialization("struct enum variants"))
    }
}

impl ser::SerializeSeq for &mut RecordSerializer {
    type Error = Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeTuple for &mut RecordSerializer {
    type Error = Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut RecordSerializer {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeTupleVariant for &mut RecordSerializer {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeMap for &mut RecordSerializer {
    type Error = Error;
    type Ok = ();

    fn serialize_key<T>(&mut self, _key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        Err(Error::UnsupportedSerialization("maps"))
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        Err(Error::UnsupportedSerialization("maps"))
    }

    fn end(self) -> Result<()> {
        Err(Error::UnsupportedSerialization("maps"))
    }
}

impl ser::SerializeStruct for &mut RecordSerializer {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        if let Some((name, params)) = self.stack.pop() {
            // restore stacked state
            let name = std::mem::replace(&mut self.name, name);
            let params = std::mem::replace(&mut self.parameters, params);
            self.parameters.push(Parameter::Typed {
                keyword: name,
                parameter: Box::new(params.into_iter().collect()),
            });
        }
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut RecordSerializer {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, to_record};
    use serde::Serialize;

    #[derive(Serialize)]
    struct LargeInteger {
        value: u64,
    }

    #[test]
    fn out_of_range_integer_returns_an_error() {
        let error = to_record(&LargeInteger { value: u64::MAX }).expect_err(
            "u64::MAX is not representable in STEP's i64 serializer",
        );
        assert!(matches!(error, Error::IntegerOutOfRange(u64::MAX)));
    }

    struct Bytes;

    impl Serialize for Bytes {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_bytes(&[1, 2, 3])
        }
    }

    #[derive(Serialize)]
    enum Shape {
        SolidBody,
    }

    #[derive(Serialize)]
    struct WithEnum {
        shape: Shape,
        visible: bool,
    }

    #[test]
    fn enum_fields_serialize_as_step_enumerations_not_strings() {
        let record = to_record(&WithEnum {
            shape: Shape::SolidBody,
            visible: true,
        })
        .unwrap();
        assert_eq!(record.name, "WITH_ENUM");
        assert_eq!(
            record.parameter,
            crate::ast::Parameter::List(vec![
                crate::ast::Parameter::Enumeration("SOLID_BODY".to_owned()),
                crate::ast::Parameter::Enumeration("TRUE".to_owned()),
            ])
        );
    }

    #[derive(Serialize)]
    struct Enclosing {
        inner: WithEnum,
    }

    #[test]
    fn nested_records_use_part_21_keywords() {
        use std::str::FromStr;

        let record = to_record(&Enclosing {
            inner: WithEnum {
                shape: Shape::SolidBody,
                visible: true,
            },
        })
        .unwrap();
        assert_eq!(
            record,
            crate::ast::Record::from_str(
                "ENCLOSING(WITH_ENUM((.SOLID_BODY.,.TRUE.)))"
            )
            .unwrap()
        );
    }

    #[test]
    fn top_level_primitives_cannot_be_step_records() {
        for error in [
            to_record(&42_i64).unwrap_err(),
            to_record(&vec![1_i64, 2]).unwrap_err(),
            to_record(&Shape::SolidBody).unwrap_err(),
        ] {
            assert!(matches!(
                error,
                Error::UnsupportedSerialization(
                    "top-level STEP records must be structs"
                )
            ));
        }
    }

    #[test]
    fn unsupported_serialization_is_an_error() {
        let error =
            to_record(&Bytes).expect_err("byte strings are unsupported");
        assert!(matches!(
            error,
            Error::UnsupportedSerialization("byte strings")
        ));
    }
}
