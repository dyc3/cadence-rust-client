//! Serialization helpers for side effects
//!
//! This module provides encoding and decoding functions for side effect
//! and mutable side effect data stored in workflow history markers.

use serde::{Deserialize, Serialize};

/// Data structure for side effect marker details
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SideEffectDetails {
    pub side_effect_id: u64,
    pub result: Vec<u8>,
}

/// Data structure for mutable side effect marker details
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MutableSideEffectDetails {
    pub id: String,
    pub result: Vec<u8>,
}

/// Data structure for version marker details
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionDetails {
    pub change_id: String,
    pub version: i32,
}

/// Encode side effect details for storage in history marker
pub fn encode_side_effect_details(side_effect_id: u64, result: &[u8]) -> Vec<u8> {
    let details = SideEffectDetails {
        side_effect_id,
        result: result.to_vec(),
    };
    serde_json::to_vec(&details).expect("Failed to encode side effect details")
}

/// Decode side effect details from history marker
pub fn decode_side_effect_details(data: &[u8]) -> Result<(u64, Vec<u8>), SideEffectError> {
    let details: SideEffectDetails = serde_json::from_slice(data)
        .map_err(|e| SideEffectError::DeserializationError(e.to_string()))?;
    Ok((details.side_effect_id, details.result))
}

/// Encode mutable side effect details for storage in history marker
pub fn encode_mutable_side_effect_details(id: &str, result: &[u8]) -> Vec<u8> {
    let details = MutableSideEffectDetails {
        id: id.to_string(),
        result: result.to_vec(),
    };
    serde_json::to_vec(&details).expect("Failed to encode mutable side effect details")
}

/// Decode mutable side effect details from history marker
pub fn decode_mutable_side_effect_details(
    data: &[u8],
) -> Result<(String, Vec<u8>), SideEffectError> {
    let details: MutableSideEffectDetails = serde_json::from_slice(data)
        .map_err(|e| SideEffectError::DeserializationError(e.to_string()))?;
    Ok((details.id, details.result))
}

/// Encode version details for storage in history marker
pub fn encode_version_details(change_id: &str, version: i32) -> Vec<u8> {
    let details = VersionDetails {
        change_id: change_id.to_string(),
        version,
    };
    serde_json::to_vec(&details).expect("Failed to encode version details")
}

/// Decode version details from history marker
pub fn decode_version_details(data: &[u8]) -> Result<(String, i32), SideEffectError> {
    let details: VersionDetails = serde_json::from_slice(data)
        .map_err(|e| SideEffectError::DeserializationError(e.to_string()))?;
    Ok((details.change_id, details.version))
}

/// Errors that can occur during side effect operations
#[derive(Debug, Clone)]
pub enum SideEffectError {
    DeserializationError(String),
}

impl std::fmt::Display for SideEffectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SideEffectError::DeserializationError(msg) => {
                write!(f, "Failed to deserialize side effect: {}", msg)
            }
        }
    }
}

impl std::error::Error for SideEffectError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_side_effect() {
        let id = 42u64;
        let result = b"test result".to_vec();
        let encoded = encode_side_effect_details(id, &result);
        let (decoded_id, decoded_result) = decode_side_effect_details(&encoded).unwrap();

        assert_eq!(id, decoded_id);
        assert_eq!(result, decoded_result);
    }

    #[test]
    fn test_encode_decode_mutable_side_effect() {
        let id = "my_mutable_id";
        let result = b"mutable result".to_vec();
        let encoded = encode_mutable_side_effect_details(id, &result);
        let (decoded_id, decoded_result) = decode_mutable_side_effect_details(&encoded).unwrap();

        assert_eq!(id, decoded_id);
        assert_eq!(result, decoded_result);
    }

    #[test]
    fn test_decode_invalid_data() {
        let invalid_data = b"not valid json";
        let result = decode_side_effect_details(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_decode_version() {
        let change_id = "my-feature-v2";
        let version = 42i32;
        let encoded = encode_version_details(change_id, version);
        let (decoded_id, decoded_version) = decode_version_details(&encoded).unwrap();
        assert_eq!(change_id, decoded_id);
        assert_eq!(version, decoded_version);
    }

    #[test]
    fn test_decode_version_invalid_data() {
        let invalid_data = b"not valid json";
        let result = decode_version_details(invalid_data);
        assert!(result.is_err());
    }
}
