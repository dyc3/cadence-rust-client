use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use deadpool_sqlite::{Config, Pool, Runtime};
use rusqlite::params;

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
    pool: Pool,
}

struct DbConn(deadpool_sqlite::Object);

#[async_trait]
impl FromResources for DbConn {
    async fn get(ctx: ResourceContext<'_>) -> Result<Self, ResourceError> {
        let pool = ctx
            .resources::<Resources>()
            .ok_or_else(|| ResourceError::new("resources not configured"))?
            .pool
            .clone();

        let conn = pool
            .get()
            .await
            .map_err(|e| ResourceError::new(format!("pool error: {e}")))?;
        Ok(DbConn(conn))
    }
}

#[activity(name = "insert_value")]
async fn insert_value(
    _ctx: &ActivityContext,
    conn: DbConn,
    value: i64,
) -> Result<i64, ActivityError> {
    let conn = conn.0;
    let inserted = conn
        .interact(move |conn| -> Result<i64, rusqlite::Error> {
            conn.execute("INSERT INTO items(value) VALUES (?1)", params![value])?;
            Ok(value)
        })
        .await
        .map_err(|e| ActivityError::ExecutionFailed(format!("join error: {e}")))?
        .map_err(|e| ActivityError::ExecutionFailed(format!("sqlite error: {e}")))?;
    Ok(inserted)
}

#[workflow(name = "check_pool")]
async fn check_pool(ctx: WorkflowContext, conn: DbConn, input: i64) -> Result<i64, WorkflowError> {
    let conn = conn.0;
    let existing = conn
        .interact(|conn| -> Result<i64, rusqlite::Error> {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM items")?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            Ok(count)
        })
        .await
        .map_err(|e| WorkflowError::ExecutionFailed(format!("join error: {e}")))?
        .map_err(|e| WorkflowError::ExecutionFailed(format!("sqlite error: {e}")))?;
    ctx.sleep(Duration::from_millis(1)).await;
    Ok(existing + input)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_deadpool_sqlite_resources_for_activity_and_workflow() {
    let mut config = Config::new("file:memdb1?mode=memory&cache=shared");
    config.pool = Some(deadpool_sqlite::PoolConfig::new(4));
    let pool = config.create_pool(Runtime::Tokio1).expect("pool");

    let conn = pool.get().await.expect("conn");
    conn.interact(|conn| conn.execute("CREATE TABLE items(value INTEGER)", []))
        .await
        .expect("create")
        .expect("create2");

    let resources = Resources { pool: pool.clone() };

    let registry = Arc::new(WorkflowRegistry::new());
    insert_value_cadence::register(registry.as_ref());
    check_pool_cadence::register(registry.as_ref());

    let activity = registry.get_activity("insert_value").expect("activity");
    let activity_ctx = ActivityContext::with_resources(
        crabdance_activity::ActivityInfo {
            activity_id: "activity-1".to_string(),
            activity_type: "insert_value".to_string(),
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

    let input = serde_json::to_vec(&41_i64).expect("input");
    let result = activity
        .execute(&activity_ctx, Some(input))
        .await
        .expect("activity ok");
    let inserted: i64 = serde_json::from_slice(&result).expect("output");
    assert_eq!(inserted, 41);

    let workflow = registry.get_workflow("check_pool").expect("workflow");
    let workflow_input = serde_json::to_vec(&1_i64).expect("workflow input");
    let ctx = WorkflowContext::new(WorkflowInfo {
        workflow_execution: WorkflowExecution::new("wf", "run"),
        workflow_type: WorkflowType {
            name: "check_pool".to_string(),
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

    let output = workflow
        .execute(ctx, Some(workflow_input))
        .await
        .expect("workflow run");
    let result: i64 = serde_json::from_slice(&output).expect("workflow output");
    assert_eq!(result, 2);
}
