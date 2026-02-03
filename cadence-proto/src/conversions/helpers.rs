//! Helper functions for common conversion operations

use crate::generated as pb;
use crate::shared as api_types;

// ============================================================================
// Duration and Timestamp Conversions
// ============================================================================

/// Convert seconds (i32) to prost Duration
pub(super) fn seconds_to_duration(seconds: Option<i32>) -> Option<prost_types::Duration> {
    seconds.map(|s| prost_types::Duration {
        seconds: s as i64,
        nanos: 0,
    })
}

/// Convert prost Duration to seconds (i32)
#[allow(dead_code)]
pub(super) fn duration_to_seconds(duration: Option<prost_types::Duration>) -> Option<i32> {
    duration.map(|d| d.seconds as i32)
}

/// Convert protobuf Timestamp to nanoseconds
pub(super) fn timestamp_to_nanos(ts: prost_types::Timestamp) -> Option<i64> {
    Some(ts.seconds * 1_000_000_000 + ts.nanos as i64)
}

/// Convert nanoseconds to protobuf Timestamp
pub(super) fn nanos_to_timestamp(nanos: i64) -> Option<prost_types::Timestamp> {
    Some(prost_types::Timestamp {
        seconds: nanos / 1_000_000_000,
        nanos: (nanos % 1_000_000_000) as i32,
    })
}

// ============================================================================
// Payload Conversions
// ============================================================================

/// Convert Vec<u8> payload to protobuf Payload
pub(super) fn bytes_to_payload(data: Option<Vec<u8>>) -> Option<pb::Payload> {
    data.map(|d| pb::Payload { data: d })
}

/// Convert protobuf Payload to Vec<u8>
#[allow(dead_code)]
pub(super) fn payload_to_bytes(payload: Option<pb::Payload>) -> Option<Vec<u8>> {
    payload.map(|p| p.data)
}

// ============================================================================
// Complex Field Conversions
// ============================================================================

/// Convert API Memo type to protobuf Memo
pub(super) fn api_memo_to_pb(memo: api_types::Memo) -> pb::Memo {
    pb::Memo {
        fields: memo
            .fields
            .into_iter()
            .map(|(k, v)| (k, pb::Payload { data: v }))
            .collect(),
    }
}

/// Convert protobuf Memo to API Memo
pub(super) fn pb_memo_to_api(memo: pb::Memo) -> api_types::Memo {
    api_types::Memo {
        fields: memo.fields.into_iter().map(|(k, v)| (k, v.data)).collect(),
    }
}

/// Convert API SearchAttributes to protobuf SearchAttributes
pub(super) fn api_search_attributes_to_pb(sa: api_types::SearchAttributes) -> pb::SearchAttributes {
    pb::SearchAttributes {
        indexed_fields: sa
            .indexed_fields
            .into_iter()
            .map(|(k, v)| (k, pb::Payload { data: v }))
            .collect(),
    }
}

/// Convert protobuf SearchAttributes to API SearchAttributes
pub(super) fn pb_search_attributes_to_api(sa: pb::SearchAttributes) -> api_types::SearchAttributes {
    api_types::SearchAttributes {
        indexed_fields: sa
            .indexed_fields
            .into_iter()
            .map(|(k, v)| (k, v.data))
            .collect(),
    }
}

/// Convert API Header to protobuf Header
pub(super) fn api_header_to_pb(header: api_types::Header) -> pb::Header {
    pb::Header {
        fields: header
            .fields
            .into_iter()
            .map(|(k, v)| (k, pb::Payload { data: v }))
            .collect(),
    }
}

/// Convert protobuf Header to API Header
pub(super) fn pb_header_to_api(h: pb::Header) -> api_types::Header {
    api_types::Header {
        fields: h.fields.into_iter().map(|(k, v)| (k, v.data)).collect(),
    }
}

/// Extract details from protobuf Failure type
pub(super) fn failure_to_details(failure: Option<pb::Failure>) -> Option<Vec<u8>> {
    failure.and_then(|f| {
        if f.details.is_empty() {
            None
        } else {
            Some(f.details)
        }
    })
}
