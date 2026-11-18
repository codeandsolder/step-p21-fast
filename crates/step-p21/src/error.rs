use serde::{de, ser};
use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    TokenizeFailed(#[from] TokenizeFailed),

    #[error("Extra input string remains behind: {0}")]
    ExtraInputRemaining(String),

    #[error("Error while deserialize STEP struct: {0}")]
    DeserializeFailed(String),

    #[error("STEP header requires three records, found {found}")]
    MissingHeaderRecords { found: usize },

    #[error("Unsupported STEP serialization: {0}")]
    UnsupportedSerialization(&'static str),

    #[error("STEP integer {0} exceeds the supported i64 range")]
    IntegerOutOfRange(u64),

    #[error("STEP serializer finished with an unclosed nested record")]
    UnbalancedRecordStack,

    #[error("Lookup failed for #{0}")]
    UnknownEntity(u64),

    #[error("Reference {0:?} is not an entity reference")]
    UnsupportedReference(crate::ast::Name),

    #[error("Entity ID #{0} is duplicated")]
    DuplicatedEntity(u64),

    #[error("Entity '{entity_name}' is not a member of the schema '{schema}'")]
    UnknownEntityName { entity_name: String, schema: String },
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::DeserializeFailed(msg.to_string())
    }
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::DeserializeFailed(msg.to_string())
    }
}

/// Error while tokenizing STEP input
pub struct TokenizeFailed {
    rendered_error: String,
}

impl fmt::Debug for TokenizeFailed {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> std::result::Result<(), fmt::Error> {
        write!(
            f,
            "Error while tokenizing STEP input\n{}",
            self.rendered_error
        )?;
        Ok(())
    }
}

// Use same output as Debug
impl fmt::Display for TokenizeFailed {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> std::result::Result<(), fmt::Error> {
        fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for TokenizeFailed {}

impl TokenizeFailed {
    #[must_use]
    pub fn new(
        input: &str,
        err: nom_language::error::VerboseError<&str>,
    ) -> Self {
        Self {
            rendered_error: nom_language::error::convert_error(input, err),
        }
    }
}
