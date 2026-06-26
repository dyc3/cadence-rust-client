//! Serialization framework for Cadence.
//!
//! This module provides traits and types for encoding and decoding data
//! passed to and from the Cadence server.

use serde::{de::DeserializeOwned, Serialize};
use std::fmt;

/// The seam for serializing workflow and activity payloads.
///
/// A single `DataConverter` is configured on both the client and the worker.
/// Cadence carries no per-payload encoding tag, so both sides must agree on the
/// converter out of band — see `CONTEXT.md`.
///
/// This trait is object-safe so it can be stored as `Arc<dyn DataConverter>`.
/// The generic, ergonomic `encode`/`decode` live on the [`DataConverterExt`]
/// extension trait, which is blanket-implemented for every converter.
pub trait DataConverter: Send + Sync {
    /// Serialize a type-erased value to its payload bytes.
    fn to_payload(&self, value: &dyn erased_serde::Serialize) -> Result<Vec<u8>, EncodingError>;

    /// Open a type-erased deserializer over payload bytes and hand it to `f`.
    ///
    /// Control is inverted (rather than returning the deserializer) because a
    /// `serde_json::Deserializer` implements `Deserializer` only through `&mut`,
    /// so it cannot be boxed and returned. The caller — which knows the concrete
    /// target type — drives deserialization inside `f` (see
    /// [`DataConverterExt::decode`]).
    fn deserialize_payload<'de>(
        &self,
        data: &'de [u8],
        f: &mut dyn FnMut(&mut dyn erased_serde::Deserializer<'de>) -> erased_serde::Result<()>,
    ) -> Result<(), EncodingError>;
}

/// Generic ergonomics over any [`DataConverter`].
///
/// Kept separate from `DataConverter` because generic methods are not
/// object-safe. Blanket-implemented for every converter, so it is available on
/// `JsonDataConverter`, `&dyn DataConverter`, and `Arc<dyn DataConverter>` alike.
pub trait DataConverterExt {
    /// Encode a serializable value to payload bytes.
    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, EncodingError>;
    /// Decode payload bytes into a value of the requested type.
    fn decode<T: DeserializeOwned>(&self, data: &[u8]) -> Result<T, EncodingError>;
}

impl<C: DataConverter + ?Sized> DataConverterExt for C {
    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, EncodingError> {
        self.to_payload(value)
    }

    fn decode<T: DeserializeOwned>(&self, data: &[u8]) -> Result<T, EncodingError> {
        let mut out: Option<T> = None;
        self.deserialize_payload(data, &mut |de| {
            out = Some(erased_serde::deserialize::<T>(de)?);
            Ok(())
        })?;
        out.ok_or_else(|| EncodingError::Deserialization("converter produced no value".to_string()))
    }
}

/// Default JSON data converter — the production adapter at the `DataConverter` seam.
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
    fn to_payload(&self, value: &dyn erased_serde::Serialize) -> Result<Vec<u8>, EncodingError> {
        let mut buf = Vec::new();
        let mut json = serde_json::Serializer::new(&mut buf);
        let mut erased = <dyn erased_serde::Serializer>::erase(&mut json);
        value
            .erased_serialize(&mut erased)
            .map_err(|e| EncodingError::Serialization(e.to_string()))?;
        Ok(buf)
    }

    fn deserialize_payload<'de>(
        &self,
        data: &'de [u8],
        f: &mut dyn FnMut(&mut dyn erased_serde::Deserializer<'de>) -> erased_serde::Result<()>,
    ) -> Result<(), EncodingError> {
        let mut json = serde_json::Deserializer::from_slice(data);
        let mut erased = <dyn erased_serde::Deserializer>::erase(&mut json);
        f(&mut erased).map_err(|e| EncodingError::Deserialization(e.to_string()))
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

/// Convenience functions — encode/decode with the default JSON converter.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, EncodingError> {
    JsonDataConverter::new().encode(value)
}

pub fn decode<T: DeserializeOwned>(data: &[u8]) -> Result<T, EncodingError> {
    JsonDataConverter::new().decode(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

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

    /// A second adapter at the `DataConverter` seam, proving it is real and
    /// swappable: it wraps JSON with a sentinel framing byte and records every
    /// call. Tests in the worker use this to assert the macro/runtime path
    /// honours the *injected* converter rather than hardcoding JSON.
    #[derive(Default)]
    struct FakeConverter {
        encodes: std::sync::atomic::AtomicUsize,
        decodes: std::sync::atomic::AtomicUsize,
    }

    impl DataConverter for FakeConverter {
        fn to_payload(
            &self,
            value: &dyn erased_serde::Serialize,
        ) -> Result<Vec<u8>, EncodingError> {
            self.encodes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut out = vec![0xFE];
            out.extend(JsonDataConverter.to_payload(value)?);
            Ok(out)
        }

        fn deserialize_payload<'de>(
            &self,
            data: &'de [u8],
            f: &mut dyn FnMut(&mut dyn erased_serde::Deserializer<'de>) -> erased_serde::Result<()>,
        ) -> Result<(), EncodingError> {
            self.decodes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let inner = data.strip_prefix(&[0xFE]).ok_or_else(|| {
                EncodingError::Deserialization("missing FakeConverter frame".to_string())
            })?;
            JsonDataConverter.deserialize_payload(inner, f)
        }
    }

    #[test]
    fn test_seam_is_object_safe_and_swappable() {
        use std::sync::atomic::Ordering;
        use std::sync::Arc;

        let fake = Arc::new(FakeConverter::default());
        // Stored and used purely as a trait object — the object-safety the seam exists for.
        let converter: Arc<dyn DataConverter> = fake.clone();

        let original = TestStruct {
            name: "seam".to_string(),
            value: 7,
        };
        let bytes = converter.encode(&original).unwrap();
        assert_eq!(bytes[0], 0xFE, "fake adapter framed the payload");

        let decoded: TestStruct = converter.decode(&bytes).unwrap();
        assert_eq!(original, decoded);

        assert_eq!(fake.encodes.load(Ordering::SeqCst), 1);
        assert_eq!(fake.decodes.load(Ordering::SeqCst), 1);

        // JSON adapter cannot read a payload written by the fake — confirms the
        // bytes really differ across adapters (a true seam, not a pass-through).
        let json: Arc<dyn DataConverter> = Arc::new(JsonDataConverter);
        assert!(json.decode::<TestStruct>(&bytes).is_err());
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
