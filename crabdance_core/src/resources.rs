//! Resource injection helpers for workflows and activities.

use async_trait::async_trait;

/// Context passed into resource guards.
#[derive(Debug, Clone, Copy)]
pub enum ResourceContext<'a> {
    Workflow {
        resources: &'a (dyn std::any::Any + Send + Sync),
    },
    Activity {
        resources: &'a (dyn std::any::Any + Send + Sync),
    },
}

impl<'a> ResourceContext<'a> {
    pub fn resources<T: 'static>(&self) -> Option<&'a T> {
        match self {
            ResourceContext::Workflow { resources } | ResourceContext::Activity { resources } => {
                resources.downcast_ref::<T>()
            }
        }
    }

    pub fn is_workflow(&self) -> bool {
        matches!(self, ResourceContext::Workflow { .. })
    }

    pub fn is_activity(&self) -> bool {
        matches!(self, ResourceContext::Activity { .. })
    }
}

/// Errors returned when acquiring resources for an activity/workflow.
#[derive(Debug, Clone, thiserror::Error)]
#[error("resource guard failed: {message}")]
pub struct ResourceError {
    message: String,
}

impl ResourceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Resource guard trait (Rocket-style).
#[async_trait]
pub trait FromResources: Sized {
    async fn get(ctx: ResourceContext<'_>) -> Result<Self, ResourceError>;
}
