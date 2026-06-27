//! Client-backed [`HistorySource`] for the worker's
//! [`WorkflowShadower`](crabdance_worker::replayer::WorkflowShadower).
//!
//! The worker crate defines the shadower and its `HistorySource` seam without
//! depending on the client; this adapter wires a live [`WorkflowClient`] into it so
//! the shadower can stream real production histories from a domain.
//!
//! ```no_run
//! # async fn demo(client: std::sync::Arc<crabdance::client::WorkflowClient>) {
//! use std::sync::Arc;
//! use crabdance::shadow::ClientHistorySource;
//! use crabdance::worker::replayer::{WorkflowReplayer, WorkflowShadower};
//! # let registry: Arc<dyn crabdance::worker::registry::Registry> = unimplemented!();
//!
//! let source = Arc::new(ClientHistorySource::new(client));
//! let replayer = WorkflowReplayer::new(registry); // workflows registered as in production
//! let shadower = WorkflowShadower::new(source, "WorkflowType = 'OrderWorkflow'", replayer);
//!
//! let reports = shadower.run().await.expect("shadow run");
//! for report in reports.iter().filter(|r| !r.is_deterministic()) {
//!     eprintln!("non-deterministic: {} / {}", report.workflow_id, report.run_id);
//! }
//! # }
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use crabdance_client::client::{Client, ListWorkflowExecutionsRequest, WorkflowClient};
use crabdance_core::CadenceError;
use crabdance_proto::shared::{HistoryEvent, HistoryEventFilterType};
use crabdance_worker::replayer::HistorySource;

/// Streams workflow executions and histories from a live domain via a [`WorkflowClient`].
pub struct ClientHistorySource {
    client: Arc<WorkflowClient>,
    page_size: i32,
}

impl ClientHistorySource {
    pub fn new(client: Arc<WorkflowClient>) -> Self {
        Self {
            client,
            page_size: 100,
        }
    }

    /// Set the visibility list page size (default 100).
    pub fn with_page_size(mut self, page_size: i32) -> Self {
        self.page_size = page_size.max(1);
        self
    }
}

#[async_trait]
impl HistorySource for ClientHistorySource {
    async fn list_executions(&self, query: &str) -> Result<Vec<(String, String)>, CadenceError> {
        let mut results = Vec::new();
        let mut next_page_token = None;

        loop {
            let response = self
                .client
                .list_workflows(ListWorkflowExecutionsRequest {
                    maximum_page_size: self.page_size,
                    next_page_token,
                    start_time_filter: None,
                    execution_filter: None,
                    type_filter: None,
                    status_filter: None,
                    query: Some(query.to_string()),
                })
                .await?;

            for info in response.executions {
                results.push((info.execution.workflow_id, info.execution.run_id));
            }

            match response.next_page_token {
                Some(token) if !token.is_empty() => next_page_token = Some(token),
                _ => break,
            }
        }

        Ok(results)
    }

    async fn fetch_history(
        &self,
        workflow_id: &str,
        run_id: &str,
    ) -> Result<Vec<HistoryEvent>, CadenceError> {
        let mut iterator = self
            .client
            .get_workflow_history(
                workflow_id,
                Some(run_id),
                false,
                HistoryEventFilterType::AllEvent,
            )
            .await?;

        let mut events = Vec::new();
        while let Some(event) = iterator.next().await? {
            events.push(event);
        }
        Ok(events)
    }
}
