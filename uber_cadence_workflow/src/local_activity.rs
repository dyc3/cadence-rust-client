//! Local activity execution support for workflows.
//!
//! Local activities are short-lived operations executed synchronously in the workflow
//! worker process without scheduling through the Cadence server. They are recorded in
//! workflow history as `MarkerRecorded` events for deterministic replay.

use serde::{Deserialize, Serialize};

/// Marker name for local activities in workflow history
pub const LOCAL_ACTIVITY_MARKER_NAME: &str = "LocalActivity";

/// Local activity marker data structure
///
/// This structure is serialized and stored in workflow history as a marker
/// to support deterministic replay of local activity results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalActivityMarkerData {
    /// Unique identifier for this local activity execution
    pub activity_id: String,

    /// Type/name of the activity
    pub activity_type: String,

    /// Serialized result if the activity succeeded
    pub result_json: Option<Vec<u8>>,

    /// Error reason if the activity failed
    pub err_reason: Option<String>,

    /// Serialized error details if the activity failed
    pub err_json: Option<Vec<u8>>,

    /// Timestamp when the activity completed (for deterministic replay)
    pub replay_time: i64,

    /// Current attempt number (starting from 0)
    pub attempt: i32,

    /// Backoff duration if retrying (in milliseconds)
    pub backoff_millis: Option<u64>,
}

impl LocalActivityMarkerData {
    /// Create a new marker data for a successful local activity execution
    pub fn success(
        activity_id: String,
        activity_type: String,
        result_json: Vec<u8>,
        replay_time: i64,
        attempt: i32,
    ) -> Self {
        Self {
            activity_id,
            activity_type,
            result_json: Some(result_json),
            err_reason: None,
            err_json: None,
            replay_time,
            attempt,
            backoff_millis: None,
        }
    }

    /// Create a new marker data for a failed local activity execution
    pub fn failure(
        activity_id: String,
        activity_type: String,
        err_reason: String,
        err_json: Option<Vec<u8>>,
        replay_time: i64,
        attempt: i32,
    ) -> Self {
        Self {
            activity_id,
            activity_type,
            result_json: None,
            err_reason: Some(err_reason),
            err_json,
            replay_time,
            attempt,
            backoff_millis: None,
        }
    }

    /// Check if this marker represents a successful execution
    pub fn is_success(&self) -> bool {
        self.result_json.is_some()
    }

    /// Check if this marker represents a failed execution
    pub fn is_failure(&self) -> bool {
        self.err_reason.is_some()
    }
}

/// Encode local activity marker data for storage in workflow history
pub fn encode_local_activity_marker(data: &LocalActivityMarkerData) -> Vec<u8> {
    serde_json::to_vec(data).expect("Failed to serialize local activity marker")
}

/// Decode local activity marker data from workflow history
pub fn decode_local_activity_marker(
    bytes: &[u8],
) -> Result<LocalActivityMarkerData, serde_json::Error> {
    serde_json::from_slice(bytes)
}

/// Convert marker data to a result for returning to workflow
pub fn marker_data_to_result(
    data: LocalActivityMarkerData,
) -> Result<Vec<u8>, crate::future::WorkflowError> {
    if let Some(result) = data.result_json {
        Ok(result)
    } else if let Some(reason) = data.err_reason {
        Err(crate::future::WorkflowError::ActivityFailed(reason))
    } else {
        Err(crate::future::WorkflowError::Generic(
            "Local activity marker has no result or error".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marker_data_success() {
        let marker = LocalActivityMarkerData::success(
            "test-activity-1".to_string(),
            "TestActivity".to_string(),
            b"result data".to_vec(),
            1234567890,
            0,
        );

        assert!(marker.is_success());
        assert!(!marker.is_failure());
        assert_eq!(marker.activity_id, "test-activity-1");
        assert_eq!(marker.activity_type, "TestActivity");
        assert_eq!(marker.result_json, Some(b"result data".to_vec()));
        assert_eq!(marker.err_reason, None);
    }

    #[test]
    fn test_marker_data_failure() {
        let marker = LocalActivityMarkerData::failure(
            "test-activity-2".to_string(),
            "TestActivity".to_string(),
            "Activity failed".to_string(),
            Some(b"error details".to_vec()),
            1234567890,
            2,
        );

        assert!(!marker.is_success());
        assert!(marker.is_failure());
        assert_eq!(marker.activity_id, "test-activity-2");
        assert_eq!(marker.err_reason, Some("Activity failed".to_string()));
        assert_eq!(marker.err_json, Some(b"error details".to_vec()));
        assert_eq!(marker.attempt, 2);
    }

    #[test]
    fn test_encode_decode_marker() {
        let original = LocalActivityMarkerData::success(
            "test-activity".to_string(),
            "TestActivity".to_string(),
            b"result".to_vec(),
            1234567890,
            0,
        );

        let encoded = encode_local_activity_marker(&original);
        let decoded = decode_local_activity_marker(&encoded).expect("Failed to decode");

        assert_eq!(decoded.activity_id, original.activity_id);
        assert_eq!(decoded.activity_type, original.activity_type);
        assert_eq!(decoded.result_json, original.result_json);
        assert_eq!(decoded.replay_time, original.replay_time);
    }

    #[test]
    fn test_marker_data_to_result_success() {
        let marker = LocalActivityMarkerData::success(
            "test".to_string(),
            "Test".to_string(),
            b"success".to_vec(),
            0,
            0,
        );

        let result = marker_data_to_result(marker);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"success");
    }

    #[test]
    fn test_marker_data_to_result_failure() {
        let marker = LocalActivityMarkerData::failure(
            "test".to_string(),
            "Test".to_string(),
            "error occurred".to_string(),
            None,
            0,
            1,
        );

        let result = marker_data_to_result(marker);
        assert!(result.is_err());
    }
}
