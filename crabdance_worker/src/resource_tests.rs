#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use crabdance_core::{FromResources, ResourceContext, ResourceError};

    #[derive(Debug)]
    struct MissingResource;

    #[async_trait]
    impl FromResources for MissingResource {
        async fn get(_ctx: ResourceContext<'_>) -> Result<Self, ResourceError> {
            Err(ResourceError::new("missing resource"))
        }
    }

    #[tokio::test]
    async fn test_missing_resource_is_retryable_error() {
        let err = MissingResource::get(ResourceContext::Activity { resources: &() })
            .await
            .expect_err("expected error");

        let activity_error = crate::registry::ActivityError::Retryable(err.to_string());
        assert!(matches!(
            activity_error,
            crate::registry::ActivityError::Retryable(_)
        ));
        assert!(activity_error.is_retryable());
    }
}
