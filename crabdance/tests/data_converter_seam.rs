//! Proves the `DataConverter` seam is honoured end-to-end by the macro/runtime
//! path — not just defined in `crabdance_core`.
//!
//! A worker can be configured with a non-JSON converter; the `#[workflow]`
//! macro must encode the workflow's output and decode its input through that
//! *injected* converter rather than the hardcoded JSON it used to emit. We prove
//! this with a framing converter (JSON wrapped in a sentinel byte) and assert
//! both that the bytes are framed and that the converter actually saw the calls.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crabdance::core::{
    DataConverter, DataConverterExt, EncodingError, JsonDataConverter, WorkflowExecution,
    WorkflowInfo, WorkflowType,
};
use crabdance::macros::workflow;
use crabdance::worker::registry::{Registry, WorkflowError, WorkflowRegistry};
use crabdance::workflow::WorkflowContext;

/// A second adapter at the `DataConverter` seam: JSON wrapped in a 0xFE frame,
/// counting every call. Stands in for a real alternative format (msgpack, etc.).
#[derive(Default)]
struct FramingConverter {
    encodes: AtomicUsize,
    decodes: AtomicUsize,
}

impl DataConverter for FramingConverter {
    fn to_payload(&self, value: &dyn erased_serde::Serialize) -> Result<Vec<u8>, EncodingError> {
        self.encodes.fetch_add(1, Ordering::SeqCst);
        let mut out = vec![0xFE];
        out.extend(JsonDataConverter.to_payload(value)?);
        Ok(out)
    }

    fn deserialize_payload<'de>(
        &self,
        data: &'de [u8],
        f: &mut dyn FnMut(&mut dyn erased_serde::Deserializer<'de>) -> erased_serde::Result<()>,
    ) -> Result<(), EncodingError> {
        self.decodes.fetch_add(1, Ordering::SeqCst);
        let inner = data
            .strip_prefix(&[0xFE])
            .ok_or_else(|| EncodingError::Deserialization("missing 0xFE frame".to_string()))?;
        JsonDataConverter.deserialize_payload(inner, f)
    }
}

#[workflow]
async fn echo_workflow(_ctx: WorkflowContext, input: String) -> Result<String, WorkflowError> {
    Ok(format!("hello {input}"))
}

fn test_workflow_info() -> WorkflowInfo {
    WorkflowInfo {
        workflow_execution: WorkflowExecution {
            workflow_id: "wf-seam".to_string(),
            run_id: "run-seam".to_string(),
        },
        workflow_type: WorkflowType {
            name: "echo_workflow".to_string(),
        },
        task_list: "seam-list".to_string(),
        start_time: chrono::Utc::now(),
        execution_start_to_close_timeout: Duration::from_secs(10),
        task_start_to_close_timeout: Duration::from_secs(5),
        attempt: 1,
        continued_execution_run_id: None,
        parent_workflow_execution: None,
        cron_schedule: None,
        memo: None,
        search_attributes: None,
    }
}

#[tokio::test]
async fn macro_path_honours_injected_converter() {
    let registry = WorkflowRegistry::new();
    echo_workflow_cadence::register(&registry);
    let wf = registry
        .get_workflow("echo_workflow")
        .expect("workflow registered");

    let converter = Arc::new(FramingConverter::default());
    let ctx = WorkflowContext::new(test_workflow_info()).with_converter(converter.clone());

    // The worker would have decoded history input through this same converter,
    // so the bytes handed to the wrapper are framed.
    let input = converter.encode(&"world".to_string()).unwrap();

    let output = wf
        .execute(ctx, Some(input))
        .await
        .expect("workflow succeeded");

    // 1. The output is framed → the macro encoded through the injected converter,
    //    not the hardcoded JsonDataConverter (which would not prepend 0xFE).
    assert_eq!(
        output.first(),
        Some(&0xFE),
        "output not framed by converter"
    );
    let decoded: String = converter.decode(&output).unwrap();
    assert_eq!(decoded, "hello world");

    // 2. The converter actually saw the input decode and output encode.
    assert!(
        converter.decodes.load(Ordering::SeqCst) >= 1,
        "input was not decoded through the injected converter"
    );
    assert!(
        converter.encodes.load(Ordering::SeqCst) >= 2,
        "output was not encoded through the injected converter"
    );

    // 3. Plain JSON cannot read the framed output — the adapters truly differ,
    //    so this is a real seam, not a pass-through.
    assert!(JsonDataConverter.decode::<String>(&output).is_err());
}
