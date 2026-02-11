use std::env;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use sqlx::Postgres;

use crabdance::activity::ActivityContext;
use crabdance::core::{
    FromResources, ResourceContext, ResourceError, WorkflowExecution, WorkflowInfo, WorkflowType,
};
use crabdance::macros::{activity, workflow};
use crabdance::worker::registry::{ActivityError, WorkflowError, WorkflowRegistry};
use crabdance::worker::Registry;
use crabdance::workflow::WorkflowContext;

#[derive(Clone)]
struct Resources {
    pool: PgPool,
}

/// A database connection that can be used in activities and workflows. It is created from the `Resources` struct which holds a connection pool.
struct DbConn {
    conn: sqlx::pool::PoolConnection<Postgres>,
}

#[async_trait]
impl FromResources for DbConn {
    async fn get(ctx: ResourceContext<'_>) -> Result<Self, ResourceError> {
        let pool = ctx
            .resources::<Resources>()
            .ok_or_else(|| ResourceError::new("resources not configured"))?
            .pool
            .clone();
        let conn = pool
            .acquire()
            .await
            .map_err(|e| ResourceError::new(format!("pool error: {e}")))?;
        Ok(DbConn { conn })
    }
}

#[activity(name = "pg_select")]
async fn pg_select(
    _ctx: &ActivityContext,
    mut db: DbConn,
    input: i64,
) -> Result<i64, ActivityError> {
    let (value,): (i64,) = sqlx::query_as("SELECT $1::BIGINT + 1")
        .bind(input)
        .fetch_one(&mut *db.conn)
        .await
        .map_err(|e| ActivityError::ExecutionFailed(format!("sqlx error: {e}")))?;
    Ok(value)
}

#[workflow(name = "pg_count")]
async fn pg_count(ctx: WorkflowContext, mut db: DbConn, input: i64) -> Result<i64, WorkflowError> {
    let (count,): (i64,) = sqlx::query_as("SELECT $1::BIGINT + 41")
        .bind(input)
        .fetch_one(&mut *db.conn)
        .await
        .map_err(|e| WorkflowError::ExecutionFailed(format!("sqlx error: {e}")))?;
    ctx.sleep(Duration::from_millis(1)).await;
    Ok(count)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_sqlx_pgpool_resources_for_activity_and_workflow() {
    let database_url = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("pg pool");

    let resources = Resources { pool: pool.clone() };

    let registry = Arc::new(WorkflowRegistry::new());
    pg_select_cadence::register(registry.as_ref());
    pg_count_cadence::register(registry.as_ref());

    let activity = registry.get_activity("pg_select").expect("activity");
    let activity_ctx = ActivityContext::with_resources(
        crabdance_activity::ActivityInfo {
            activity_id: "activity-1".to_string(),
            activity_type: "pg_select".to_string(),
            task_token: vec![],
            workflow_execution: crabdance_activity::WorkflowExecution::new("wf", "run"),
            attempt: 1,
            scheduled_time: Utc::now(),
            started_time: Utc::now(),
            deadline: None,
            heartbeat_timeout: Duration::from_secs(0),
            heartbeat_details: None,
        },
        None,
        Arc::new(resources.clone()),
    );

    let input_bytes = serde_json::to_vec(&1_i64).expect("input");
    let result = activity
        .execute(&activity_ctx, Some(input_bytes))
        .await
        .expect("activity ok");
    let inserted: i64 = serde_json::from_slice(&result).expect("output");
    assert_eq!(inserted, 2);

    let workflow = registry.get_workflow("pg_count").expect("workflow");
    let ctx = WorkflowContext::new(WorkflowInfo {
        workflow_execution: WorkflowExecution::new("wf", "run"),
        workflow_type: WorkflowType {
            name: "pg_count".to_string(),
        },
        task_list: "test-task-list".to_string(),
        start_time: Utc::now(),
        execution_start_to_close_timeout: Duration::from_secs(10),
        task_start_to_close_timeout: Duration::from_secs(10),
        attempt: 1,
        continued_execution_run_id: None,
        parent_workflow_execution: None,
        cron_schedule: None,
        memo: None,
        search_attributes: None,
    })
    .with_resources(Some(Arc::new(resources)));

    let workflow_input = serde_json::to_vec(&1_i64).expect("workflow input");
    let output = workflow
        .execute(ctx, Some(workflow_input))
        .await
        .expect("workflow run");
    let count: i64 = serde_json::from_slice(&output).expect("workflow output");
    assert_eq!(count, 42);
}
