//! Conversions for visibility operations (scan, count, search attributes)

use crate::generated as pb;
use crate::workflow_service as api;

use super::workflow::pb_workflow_execution_info_to_api;

// ============================================================================
// Scan Workflow Executions
// ============================================================================

impl From<api::ScanWorkflowExecutionsRequest> for pb::ScanWorkflowExecutionsRequest {
    fn from(req: api::ScanWorkflowExecutionsRequest) -> Self {
        pb::ScanWorkflowExecutionsRequest {
            domain: req.domain,
            page_size: req.page_size,
            next_page_token: req.next_page_token.unwrap_or_default(),
            query: req.query.unwrap_or_default(),
        }
    }
}

impl From<pb::ScanWorkflowExecutionsResponse> for api::ScanWorkflowExecutionsResponse {
    fn from(resp: pb::ScanWorkflowExecutionsResponse) -> Self {
        api::ScanWorkflowExecutionsResponse {
            executions: resp
                .executions
                .into_iter()
                .map(pb_workflow_execution_info_to_api)
                .collect(),
            next_page_token: if resp.next_page_token.is_empty() {
                None
            } else {
                Some(resp.next_page_token)
            },
        }
    }
}

// ============================================================================
// Count Workflow Executions
// ============================================================================

impl From<api::CountWorkflowExecutionsRequest> for pb::CountWorkflowExecutionsRequest {
    fn from(req: api::CountWorkflowExecutionsRequest) -> Self {
        pb::CountWorkflowExecutionsRequest {
            domain: req.domain,
            query: req.query.unwrap_or_default(),
        }
    }
}

impl From<pb::CountWorkflowExecutionsResponse> for api::CountWorkflowExecutionsResponse {
    fn from(resp: pb::CountWorkflowExecutionsResponse) -> Self {
        api::CountWorkflowExecutionsResponse { count: resp.count }
    }
}

// ============================================================================
// Get Search Attributes
// ============================================================================

impl From<api::GetSearchAttributesRequest> for pb::GetSearchAttributesRequest {
    fn from(_req: api::GetSearchAttributesRequest) -> Self {
        pb::GetSearchAttributesRequest {}
    }
}

impl From<pb::GetSearchAttributesResponse> for api::GetSearchAttributesResponse {
    fn from(resp: pb::GetSearchAttributesResponse) -> Self {
        let keys = resp
            .keys
            .into_iter()
            .map(|(k, v)| (k, indexed_value_type_from_pb(v)))
            .collect();
        api::GetSearchAttributesResponse { keys }
    }
}

fn indexed_value_type_from_pb(value: i32) -> api::IndexedValueType {
    match value {
        0 => api::IndexedValueType::String,
        1 => api::IndexedValueType::Keyword,
        2 => api::IndexedValueType::Int,
        3 => api::IndexedValueType::Double,
        4 => api::IndexedValueType::Bool,
        5 => api::IndexedValueType::Datetime,
        _ => api::IndexedValueType::String, // Default to String for unknown types
    }
}
