//! Custom data converters for serialization.

use serde::{Deserialize, Serialize};

/// Custom JSON data converter
pub struct JsonDataConverter;

impl JsonDataConverter {
    pub fn new() -> Self {
        Self
    }

    pub fn to_payload<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(value)
    }

    pub fn from_payload<T: for<'de> Deserialize<'de>>(
        &self,
        payload: &[u8],
    ) -> Result<T, serde_json::Error> {
        serde_json::from_slice(payload)
    }
}
