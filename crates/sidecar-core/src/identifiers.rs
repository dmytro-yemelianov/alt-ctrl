use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

macro_rules! string_identifier {
    ($name:ident) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates an identifier after rejecting an empty or whitespace-only
            /// value.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidIdentifier`] when the supplied value is blank.
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidIdentifier> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(InvalidIdentifier);
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

string_identifier!(ActionId);
string_identifier!(EventId);
string_identifier!(QuestionId);
string_identifier!(SessionId);
string_identifier!(ToolInvocationId);

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("identifier must not be empty")]
pub struct InvalidIdentifier;

#[cfg(test)]
mod tests {
    use super::{InvalidIdentifier, SessionId};

    #[test]
    fn identifier_rejects_blank_values() {
        assert_eq!(SessionId::new(" \t"), Err(InvalidIdentifier));
    }

    #[test]
    fn identifier_preserves_non_blank_value() {
        let id = SessionId::new("session-01").expect("valid identifier");
        assert_eq!(id.as_str(), "session-01");
        assert_eq!(id.to_string(), "session-01");
    }
}
