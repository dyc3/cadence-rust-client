//! gRPC client implementation for Cadence workflow service.
//!
//! This module provides the gRPC-based client for communicating with the
//! Cadence server using protocol buffers.

use async_trait::async_trait;
use cadence_core::CadenceError;
use cadence_proto::generated::domain_api_client::DomainApiClient;
use cadence_proto::generated::visibility_api_client::VisibilityApiClient;
use cadence_proto::generated::worker_api_client::WorkerApiClient;
use cadence_proto::generated::workflow_api_client::WorkflowApiClient;
use cadence_proto::workflow_service::*;
use tonic::transport::Channel;
use tonic::{metadata::MetadataValue, Request, Status};

use crate::auth::{AuthInterceptor, BoxedAuthProvider};

/// Library version sent to Cadence server in headers
const LIBRARY_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Feature version indicates client feature capabilities
const FEATURE_VERSION: &str = "1.0.0";

/// Client implementation identifier
const CLIENT_IMPL_NAME: &str = "cadence-rust";

/// Interceptor function that adds Cadence-required headers to all gRPC requests
///
/// These headers are required by the Cadence server to identify the client
/// and its capabilities. They match the headers sent by the Go client.
#[expect(clippy::result_large_err)]
fn add_cadence_headers(mut req: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
    let metadata = req.metadata_mut();

    // Add Cadence client identification headers (matching Go client)
    metadata.insert(
        "cadence-client-library-version",
        MetadataValue::from_static(LIBRARY_VERSION),
    );
    metadata.insert(
        "cadence-client-feature-version",
        MetadataValue::from_static(FEATURE_VERSION),
    );
    metadata.insert(
        "cadence-client-name",
        MetadataValue::from_static(CLIENT_IMPL_NAME),
    );
    metadata.insert("cadence-caller-type", MetadataValue::from_static("sdk"));

    // Add YARPC-required context headers for gRPC transport
    // These are the standard YARPC headers that the server expects
    metadata.insert(
        "rpc-service",
        MetadataValue::from_static("cadence-frontend"),
    );
    metadata.insert("rpc-caller", MetadataValue::from_static(CLIENT_IMPL_NAME));
    metadata.insert("rpc-encoding", MetadataValue::from_static("proto"));

    // YARPC expects TTL in the standard gRPC timeout header
    // Format: value followed by time unit (H, M, S, m, u, n)
    // Using 60S for 60 seconds
    metadata.insert("grpc-timeout", MetadataValue::from_static("60S"));

    Ok(req)
}

// Chained interceptors: Auth -> Headers -> Channel
// AuthInterceptor is innermost (called first), then headers interceptor
pub type InterceptedChannel = tonic::service::interceptor::InterceptedService<
    tonic::service::interceptor::InterceptedService<Channel, AuthInterceptor>,
    fn(tonic::Request<()>) -> Result<tonic::Request<()>, Status>,
>;

/// gRPC-based workflow service client
#[derive(Clone)]
pub struct GrpcWorkflowServiceClient {
    workflow_client: WorkflowApiClient<InterceptedChannel>,
    worker_client: WorkerApiClient<InterceptedChannel>,
    visibility_client: VisibilityApiClient<InterceptedChannel>,
    domain_client: DomainApiClient<InterceptedChannel>,
    domain: String,
}

impl GrpcWorkflowServiceClient {
    /// Create a new gRPC client by connecting to the specified endpoint
    ///
    /// # Arguments
    /// * `endpoint` - The gRPC endpoint URL (e.g., "http://localhost:7833")
    /// * `domain` - The Cadence domain to use
    /// * `auth_provider` - Optional authentication provider for JWT/OAuth
    pub async fn connect(
        endpoint: impl Into<String>,
        domain: impl Into<String>,
        auth_provider: Option<BoxedAuthProvider>,
    ) -> Result<Self, CadenceError> {
        let endpoint = endpoint.into();

        let channel = Channel::from_shared(endpoint.clone())
            .map_err(|e| CadenceError::Transport(format!("Invalid endpoint: {}", e)))?
            .connect()
            .await
            .map_err(|e| CadenceError::Transport(e.to_string()))?;

        // Layer 1: Auth interceptor (innermost)
        let auth_interceptor = AuthInterceptor::new(auth_provider);
        let channel_with_auth =
            tonic::service::interceptor::InterceptedService::new(channel, auth_interceptor);

        // Layer 2: Cadence headers (outermost)
        let header_fn = add_cadence_headers as fn(Request<()>) -> Result<Request<()>, Status>;
        let channel_with_headers =
            tonic::service::interceptor::InterceptedService::new(channel_with_auth, header_fn);

        // Create API clients with chained interceptors
        let workflow_client = WorkflowApiClient::new(channel_with_headers.clone());
        let worker_client = WorkerApiClient::new(channel_with_headers.clone());
        let visibility_client = VisibilityApiClient::new(channel_with_headers.clone());
        let domain_client = DomainApiClient::new(channel_with_headers);

        Ok(Self {
            workflow_client,
            worker_client,
            visibility_client,
            domain_client,
            domain: domain.into(),
        })
    }

    /// Get the domain name this client is configured for
    pub fn domain(&self) -> &str {
        &self.domain
    }
}

#[async_trait]
impl WorkflowService for GrpcWorkflowServiceClient {
    type Error = CadenceError;

    async fn start_workflow_execution(
        &self,
        request: StartWorkflowExecutionRequest,
    ) -> Result<StartWorkflowExecutionResponse, Self::Error> {
        // Convert our API request type to protobuf type
        let pb_request: cadence_proto::generated::StartWorkflowExecutionRequest = request.into();

        // Make the gRPC call
        let mut client = self.workflow_client.clone();
        let response = client
            .start_workflow_execution(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        // Convert protobuf response back to our API type
        Ok(response.into_inner().into())
    }

    async fn signal_workflow_execution(
        &self,
        request: SignalWorkflowExecutionRequest,
    ) -> Result<SignalWorkflowExecutionResponse, Self::Error> {
        // Convert our API request type to protobuf type
        let pb_request: cadence_proto::generated::SignalWorkflowExecutionRequest = request.into();

        // Make the gRPC call
        let mut client = self.workflow_client.clone();
        let response = client
            .signal_workflow_execution(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        // Convert protobuf response back to our API type
        Ok(response.into_inner().into())
    }

    async fn signal_with_start_workflow_execution(
        &self,
        request: SignalWithStartWorkflowExecutionRequest,
    ) -> Result<StartWorkflowExecutionResponse, Self::Error> {
        let pb_request: cadence_proto::generated::SignalWithStartWorkflowExecutionRequest =
            request.into();

        let mut client = self.workflow_client.clone();
        let response = client
            .signal_with_start_workflow_execution(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn request_cancel_workflow_execution(
        &self,
        request: RequestCancelWorkflowExecutionRequest,
    ) -> Result<RequestCancelWorkflowExecutionResponse, Self::Error> {
        let pb_request: cadence_proto::generated::RequestCancelWorkflowExecutionRequest =
            request.into();

        let mut client = self.workflow_client.clone();
        let response = client
            .request_cancel_workflow_execution(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn terminate_workflow_execution(
        &self,
        request: TerminateWorkflowExecutionRequest,
    ) -> Result<TerminateWorkflowExecutionResponse, Self::Error> {
        let pb_request: cadence_proto::generated::TerminateWorkflowExecutionRequest =
            request.into();

        let mut client = self.workflow_client.clone();
        let response = client
            .terminate_workflow_execution(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn query_workflow(
        &self,
        request: QueryWorkflowRequest,
    ) -> Result<QueryWorkflowResponse, Self::Error> {
        let pb_request: cadence_proto::generated::QueryWorkflowRequest = request.into();

        let mut client = self.workflow_client.clone();
        let response = client
            .query_workflow(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn poll_for_decision_task(
        &self,
        request: PollForDecisionTaskRequest,
    ) -> Result<PollForDecisionTaskResponse, Self::Error> {
        let pb_request: cadence_proto::generated::PollForDecisionTaskRequest = request.into();

        let mut client = self.worker_client.clone();
        let response = client
            .poll_for_decision_task(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn respond_decision_task_completed(
        &self,
        request: RespondDecisionTaskCompletedRequest,
    ) -> Result<RespondDecisionTaskCompletedResponse, Self::Error> {
        let pb_request: cadence_proto::generated::RespondDecisionTaskCompletedRequest =
            request.into();

        let mut client = self.worker_client.clone();
        let response = client
            .respond_decision_task_completed(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn respond_decision_task_failed(
        &self,
        request: RespondDecisionTaskFailedRequest,
    ) -> Result<RespondDecisionTaskFailedResponse, Self::Error> {
        let pb_request: cadence_proto::generated::RespondDecisionTaskFailedRequest = request.into();

        let mut client = self.worker_client.clone();
        let response = client
            .respond_decision_task_failed(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn poll_for_activity_task(
        &self,
        request: PollForActivityTaskRequest,
    ) -> Result<PollForActivityTaskResponse, Self::Error> {
        let pb_request: cadence_proto::generated::PollForActivityTaskRequest = request.into();

        let mut client = self.worker_client.clone();
        let response = client
            .poll_for_activity_task(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn record_activity_task_heartbeat(
        &self,
        request: RecordActivityTaskHeartbeatRequest,
    ) -> Result<RecordActivityTaskHeartbeatResponse, Self::Error> {
        let pb_request: cadence_proto::generated::RecordActivityTaskHeartbeatRequest =
            request.into();

        let mut client = self.worker_client.clone();
        let response = client
            .record_activity_task_heartbeat(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn respond_activity_task_completed(
        &self,
        request: RespondActivityTaskCompletedRequest,
    ) -> Result<RespondActivityTaskCompletedResponse, Self::Error> {
        let pb_request: cadence_proto::generated::RespondActivityTaskCompletedRequest =
            request.into();

        let mut client = self.worker_client.clone();
        let response = client
            .respond_activity_task_completed(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn respond_activity_task_failed(
        &self,
        request: RespondActivityTaskFailedRequest,
    ) -> Result<RespondActivityTaskFailedResponse, Self::Error> {
        let pb_request: cadence_proto::generated::RespondActivityTaskFailedRequest = request.into();

        let mut client = self.worker_client.clone();
        let response = client
            .respond_activity_task_failed(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn get_workflow_execution_history(
        &self,
        request: GetWorkflowExecutionHistoryRequest,
    ) -> Result<GetWorkflowExecutionHistoryResponse, Self::Error> {
        let pb_request: cadence_proto::generated::GetWorkflowExecutionHistoryRequest =
            request.into();

        let mut client = self.workflow_client.clone();
        let response = client
            .get_workflow_execution_history(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn describe_workflow_execution(
        &self,
        request: DescribeWorkflowExecutionRequest,
    ) -> Result<DescribeWorkflowExecutionResponse, Self::Error> {
        let pb_request: cadence_proto::generated::DescribeWorkflowExecutionRequest = request.into();

        let mut client = self.workflow_client.clone();
        let response = client
            .describe_workflow_execution(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;

        Ok(response.into_inner().into())
    }

    async fn list_open_workflow_executions(
        &self,
        request: ListOpenWorkflowExecutionsRequest,
    ) -> Result<ListOpenWorkflowExecutionsResponse, Self::Error> {
        let pb_request: cadence_proto::generated::ListOpenWorkflowExecutionsRequest =
            request.into();
        let mut client = self.visibility_client.clone();
        let response = client
            .list_open_workflow_executions(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn list_closed_workflow_executions(
        &self,
        request: ListClosedWorkflowExecutionsRequest,
    ) -> Result<ListClosedWorkflowExecutionsResponse, Self::Error> {
        let pb_request: cadence_proto::generated::ListClosedWorkflowExecutionsRequest =
            request.into();
        let mut client = self.visibility_client.clone();
        let response = client
            .list_closed_workflow_executions(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn register_domain(&self, request: RegisterDomainRequest) -> Result<(), Self::Error> {
        let pb_request: cadence_proto::generated::RegisterDomainRequest = request.into();
        let mut client = self.domain_client.clone();
        client
            .register_domain(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(())
    }

    async fn describe_domain(
        &self,
        request: DescribeDomainRequest,
    ) -> Result<DescribeDomainResponse, Self::Error> {
        let pb_request: cadence_proto::generated::DescribeDomainRequest = request.into();
        let mut client = self.domain_client.clone();
        let response = client
            .describe_domain(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn update_domain(
        &self,
        request: UpdateDomainRequest,
    ) -> Result<UpdateDomainResponse, Self::Error> {
        let pb_request: cadence_proto::generated::UpdateDomainRequest = request.into();
        let mut client = self.domain_client.clone();
        let response = client
            .update_domain(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn failover_domain(
        &self,
        request: FailoverDomainRequest,
    ) -> Result<FailoverDomainResponse, Self::Error> {
        let pb_request: cadence_proto::generated::FailoverDomainRequest = request.into();
        let mut client = self.domain_client.clone();
        client
            .failover_domain(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(FailoverDomainResponse {})
    }

    async fn respond_activity_task_completed_by_id(
        &self,
        request: RespondActivityTaskCompletedByIdRequest,
    ) -> Result<RespondActivityTaskCompletedByIdResponse, Self::Error> {
        let pb_request: cadence_proto::generated::RespondActivityTaskCompletedByIdRequest =
            request.into();
        let mut client = self.worker_client.clone();
        let response = client
            .respond_activity_task_completed_by_id(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn respond_activity_task_failed_by_id(
        &self,
        request: RespondActivityTaskFailedByIdRequest,
    ) -> Result<RespondActivityTaskFailedByIdResponse, Self::Error> {
        let pb_request: cadence_proto::generated::RespondActivityTaskFailedByIdRequest =
            request.into();
        let mut client = self.worker_client.clone();
        let response = client
            .respond_activity_task_failed_by_id(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn respond_activity_task_canceled_by_id(
        &self,
        request: RespondActivityTaskCanceledByIdRequest,
    ) -> Result<RespondActivityTaskCanceledByIdResponse, Self::Error> {
        let pb_request: cadence_proto::generated::RespondActivityTaskCanceledByIdRequest =
            request.into();
        let mut client = self.worker_client.clone();
        let response = client
            .respond_activity_task_canceled_by_id(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn record_activity_task_heartbeat_by_id(
        &self,
        request: RecordActivityTaskHeartbeatByIdRequest,
    ) -> Result<RecordActivityTaskHeartbeatByIdResponse, Self::Error> {
        let pb_request: cadence_proto::generated::RecordActivityTaskHeartbeatByIdRequest =
            request.into();
        let mut client = self.worker_client.clone();
        let response = client
            .record_activity_task_heartbeat_by_id(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn scan_workflow_executions(
        &self,
        request: ScanWorkflowExecutionsRequest,
    ) -> Result<ScanWorkflowExecutionsResponse, Self::Error> {
        let pb_request: cadence_proto::generated::ScanWorkflowExecutionsRequest = request.into();
        let mut client = self.visibility_client.clone();
        let response = client
            .scan_workflow_executions(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn count_workflow_executions(
        &self,
        request: CountWorkflowExecutionsRequest,
    ) -> Result<CountWorkflowExecutionsResponse, Self::Error> {
        let pb_request: cadence_proto::generated::CountWorkflowExecutionsRequest = request.into();
        let mut client = self.visibility_client.clone();
        let response = client
            .count_workflow_executions(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn get_search_attributes(
        &self,
        request: GetSearchAttributesRequest,
    ) -> Result<GetSearchAttributesResponse, Self::Error> {
        let pb_request: cadence_proto::generated::GetSearchAttributesRequest = request.into();
        let mut client = self.visibility_client.clone();
        let response = client
            .get_search_attributes(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn reset_workflow_execution(
        &self,
        request: ResetWorkflowExecutionRequest,
    ) -> Result<ResetWorkflowExecutionResponse, Self::Error> {
        let pb_request: cadence_proto::generated::ResetWorkflowExecutionRequest = request.into();
        let mut client = self.workflow_client.clone();
        let response = client
            .reset_workflow_execution(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn describe_task_list(
        &self,
        request: DescribeTaskListRequest,
    ) -> Result<DescribeTaskListResponse, Self::Error> {
        let pb_request: cadence_proto::generated::DescribeTaskListRequest = request.into();
        let mut client = self.workflow_client.clone();
        let response = client
            .describe_task_list(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }

    async fn refresh_workflow_tasks(
        &self,
        request: RefreshWorkflowTasksRequest,
    ) -> Result<RefreshWorkflowTasksResponse, Self::Error> {
        let pb_request: cadence_proto::generated::RefreshWorkflowTasksRequest = request.into();
        let mut client = self.workflow_client.clone();
        let response = client
            .refresh_workflow_tasks(pb_request)
            .await
            .map_err(|e| CadenceError::Transport(format!("gRPC error: {}", e)))?;
        Ok(response.into_inner().into())
    }
}
