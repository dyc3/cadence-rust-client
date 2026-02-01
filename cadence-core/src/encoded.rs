//! Serialization framework for Cadence.
//!
//! This module provides traits and types for encoding and decoding data
//! passed to and from the Cadence server.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;

/// Trait for data converters/serializers
pub trait DataConverter: Send + Sync {
    /// Encode a value to bytes
    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, EncodingError>;
    /// Decode bytes to a value
    fn decode<T: DeserializeOwned>(&self, data: &[u8]) -> Result<T, EncodingError>;
}

/// Default JSON data converter
pub struct JsonDataConverter;

impl JsonDataConverter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonDataConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl DataConverter for JsonDataConverter {
    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, EncodingError> {
        serde_json::to_vec(value).map_err(|e| EncodingError::Serialization(e.to_string()))
    }

    fn decode<T: DeserializeOwned>(&self, data: &[u8]) -> Result<T, EncodingError> {
        serde_json::from_slice(data).map_err(|e| EncodingError::Deserialization(e.to_string()))
    }
}

/// Encoding errors
#[derive(Debug, Clone, PartialEq)]
pub enum EncodingError {
    Serialization(String),
    Deserialization(String),
    UnsupportedEncoding(String),
}

impl fmt::Display for EncodingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodingError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            EncodingError::Deserialization(msg) => write!(f, "Deserialization error: {}", msg),
            EncodingError::UnsupportedEncoding(enc) => {
                write!(f, "Unsupported encoding: {}", enc)
            }
        }
    }
}

impl std::error::Error for EncodingError {}

/// Encoded value that can be decoded later
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedValue {
    data: Vec<u8>,
    encoding: Encoding,
}

impl EncodedValue {
    /// Create a new encoded value
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            encoding: Encoding::Json,
        }
    }

    /// Create from raw bytes with specified encoding
    pub fn with_encoding(data: Vec<u8>, encoding: Encoding) -> Self {
        Self { data, encoding }
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get the encoding type
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Decode to a typed value
    pub fn decode<T: DeserializeOwned>(&self) -> Result<T, EncodingError> {
        match self.encoding {
            Encoding::Json => serde_json::from_slice(&self.data)
                .map_err(|e| EncodingError::Deserialization(e.to_string())),
            Encoding::Raw => Err(EncodingError::UnsupportedEncoding(
                "Cannot decode raw bytes".to_string(),
            )),
        }
    }

    /// Get length of encoded data
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if encoded data is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Encoding types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Json,
    Raw,
}

/// Encoded values for multiple arguments
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedValues {
    data: Vec<u8>,
}

impl EncodedValues {
    /// Create new encoded values
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Decode to multiple typed values
    pub fn decode<T: DeserializeOwned>(&self) -> Result<T, EncodingError> {
        serde_json::from_slice(&self.data)
            .map_err(|e| EncodingError::Deserialization(e.to_string()))
    }
}

/// Convenience functions
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, EncodingError> {
    JsonDataConverter::new().encode(value)
}

pub fn decode<T: DeserializeOwned>(data: &[u8]) -> Result<T, EncodingError> {
    JsonDataConverter::new().decode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        value: i32,
    }

    #[test]
    fn test_json_encode_decode() {
        let converter = JsonDataConverter::new();
        let original = TestStruct {
            name: "test".to_string(),
            value: 42,
        };

        let encoded = converter.encode(&original).unwrap();
        let decoded: TestStruct = converter.decode(&encoded).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encoded_value() {
        let original = TestStruct {
            name: "test".to_string(),
            value: 42,
        };

        let data = encode(&original).unwrap();
        let encoded = EncodedValue::new(data);

        let decoded: TestStruct = encoded.decode().unwrap();
        assert_eq!(original, decoded);
    }
}
