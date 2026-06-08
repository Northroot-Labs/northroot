use crate::digest::Digest;
use crate::validation::ValidationError;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};

/// Opaque reference to content-addressed bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentRef {
    /// Digest that identifies the referenced bytes.
    pub digest: Digest,
    /// Optional size hint; does not affect hashing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// Optional media type hint (e.g., `application/json`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

macro_rules! newtype {
    ($name:ident, $doc:expr, $pattern:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Parses a validated identifier from a string.
            pub fn parse(value: impl Into<String>) -> Result<Self, ValidationError> {
                let s = value.into();
                if !Regex::new($pattern).expect("invalid regex").is_match(&s) {
                    return Err(ValidationError::PatternMismatch {
                        field: stringify!($name),
                        value: s,
                    });
                }
                Ok(Self(s))
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }

        impl TryFrom<String> for $name {
            type Error = ValidationError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ValidationError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

newtype!(
    ProfileId,
    "Identifier for canonicalization profiles (pattern: `[A-Za-z0-9_-]{16,128}`)",
    r"^[A-Za-z0-9_-]{16,128}$"
);
newtype!(
    PrincipalId,
    "Stable identifier for principals (`kind:name`, lowercase, URL-safe).",
    r"^(human|service|agent|org):[a-z][a-z0-9_-]{0,62}$"
);
newtype!(
    ToolName,
    "Canonical tool identifier like `canon.hash` or `llm.generate`.",
    r"^[a-z][a-z0-9_]*([.][a-z][a-z0-9_]*){0,7}$"
);
newtype!(
    Timestamp,
    "UTC RFC3339 timestamp with `Z` suffix.",
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,9})?Z$"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_deserialization_validates_identifier_patterns() {
        let profile: ProfileId = serde_json::from_str(r#""northroot-canonical-v1""#).unwrap();
        assert_eq!(profile.as_ref(), "northroot-canonical-v1");

        let invalid_profile = serde_json::from_str::<ProfileId>(r#""short""#);
        assert!(invalid_profile.is_err());

        let invalid_principal = serde_json::from_str::<PrincipalId>(r#""service:UPPER""#);
        assert!(invalid_principal.is_err());
    }

    #[test]
    fn try_from_rejects_nonconforming_identifiers() {
        assert!(PrincipalId::try_from("service:valid_agent-1").is_ok());
        assert!(PrincipalId::try_from("service:Invalid").is_err());
        assert!(Timestamp::try_from("2024-01-01T00:00:00Z").is_ok());
        assert!(Timestamp::try_from("2024-01-01 00:00:00").is_err());
    }
}
