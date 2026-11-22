use std::fmt;

/// Error while tokenizing STEP input
pub struct TokenizeFailed {
    rendered_error: String,
}

impl fmt::Debug for TokenizeFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
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
