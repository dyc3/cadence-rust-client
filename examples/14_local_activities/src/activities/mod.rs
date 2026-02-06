//! Activity implementations for local activities example.
//!
//! These activities are designed to be executed as local activities
//! (synchronously in the workflow thread).

use cadence_activity::ActivityContext;
use cadence_worker::ActivityError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Data validation activity - fast operation ideal for local activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateInput {
    pub data: serde_json::Value,
    pub schema: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateOutput {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub async fn validate_data_activity(
    _ctx: &ActivityContext,
    input: ValidateInput,
) -> Result<ValidateOutput, ActivityError> {
    info!("Validating data against schema: {}", input.schema);

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Simulate fast validation
    if input.data.is_null() {
        errors.push("Data cannot be null".to_string());
    }

    if input.schema.is_empty() {
        warnings.push("No schema specified, using default validation".to_string());
    }

    // Quick validation check
    if let Some(obj) = input.data.as_object() {
        if obj.is_empty() {
            warnings.push("Empty data object".to_string());
        }
    }

    tokio::time::sleep(Duration::from_millis(5)).await;

    let valid = errors.is_empty();

    info!(
        "Validation complete: valid={}, errors={}, warnings={}",
        valid,
        errors.len(),
        warnings.len()
    );

    Ok(ValidateOutput {
        valid,
        errors,
        warnings,
    })
}

/// Data transformation activity - another good local activity candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformInput {
    pub data: serde_json::Value,
    pub transform_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformOutput {
    pub transformed_data: serde_json::Value,
    pub transform_applied: String,
}

pub async fn transform_data_activity(
    _ctx: &ActivityContext,
    input: TransformInput,
) -> Result<TransformOutput, ActivityError> {
    info!("Applying transform: {}", input.transform_type);

    let transformed = match input.transform_type.as_str() {
        "uppercase" => {
            if let Some(s) = input.data.as_str() {
                serde_json::Value::String(s.to_uppercase())
            } else {
                input.data.clone()
            }
        }
        "lowercase" => {
            if let Some(s) = input.data.as_str() {
                serde_json::Value::String(s.to_lowercase())
            } else {
                input.data.clone()
            }
        }
        "add_timestamp" => {
            let mut obj = input.data.clone();
            if let Some(map) = obj.as_object_mut() {
                let mut new_map = map.clone();
                new_map.insert(
                    "timestamp".to_string(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                serde_json::Value::Object(new_map)
            } else {
                input.data.clone()
            }
        }
        _ => input.data.clone(),
    };

    tokio::time::sleep(Duration::from_millis(3)).await;

    info!("Transform applied successfully");

    Ok(TransformOutput {
        transformed_data: transformed,
        transform_applied: input.transform_type,
    })
}

/// Enrichment activity - fast data enrichment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichInput {
    pub record: serde_json::Value,
    pub enrichment_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichOutput {
    pub enriched_record: serde_json::Value,
    pub fields_added: Vec<String>,
}

pub async fn enrich_data_activity(
    _ctx: &ActivityContext,
    input: EnrichInput,
) -> Result<EnrichOutput, ActivityError> {
    info!(
        "Enriching record with {} fields",
        input.enrichment_fields.len()
    );

    let mut record = input.record.clone();
    let mut fields_added = Vec::new();

    if let Some(obj) = record.as_object_mut() {
        let mut new_obj = obj.clone();

        for field in &input.enrichment_fields {
            match field.as_str() {
                "uuid" => {
                    new_obj.insert(
                        "uuid".to_string(),
                        serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
                    );
                    fields_added.push("uuid".to_string());
                }
                "processed_at" => {
                    new_obj.insert(
                        "processed_at".to_string(),
                        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                    );
                    fields_added.push("processed_at".to_string());
                }
                "version" => {
                    new_obj.insert(
                        "version".to_string(),
                        serde_json::Value::String("1.0.0".to_string()),
                    );
                    fields_added.push("version".to_string());
                }
                _ => {}
            }
        }

        record = serde_json::Value::Object(new_obj);
    }

    tokio::time::sleep(Duration::from_millis(2)).await;

    info!("Enrichment complete, added {} fields", fields_added.len());

    Ok(EnrichOutput {
        enriched_record: record,
        fields_added,
    })
}

/// Decision activity - fast business logic decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionInput {
    pub context: serde_json::Value,
    pub decision_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutput {
    pub decision: String,
    pub confidence: f64,
    pub reasoning: Vec<String>,
}

pub async fn make_decision_activity(
    _ctx: &ActivityContext,
    input: DecisionInput,
) -> Result<DecisionOutput, ActivityError> {
    info!("Making decision of type: {}", input.decision_type);

    let (decision, confidence, reasoning) = match input.decision_type.as_str() {
        "approve" => {
            // Check approval criteria
            let reasons = vec!["Criteria check passed".to_string()];
            let conf = 0.95;
            let dec = if conf > 0.9 { "APPROVED" } else { "REJECTED" };
            (dec.to_string(), conf, reasons)
        }
        "route" => {
            let reasons = vec!["Default routing".to_string()];
            ("ROUTE_A".to_string(), 0.85, reasons)
        }
        "classify" => {
            let reasons = vec!["Classification complete".to_string()];
            ("CLASS_B".to_string(), 0.92, reasons)
        }
        _ => (
            "UNKNOWN".to_string(),
            0.0,
            vec!["Unknown decision type".to_string()],
        ),
    };

    tokio::time::sleep(Duration::from_millis(1)).await;

    info!("Decision made: {} (confidence: {})", decision, confidence);

    Ok(DecisionOutput {
        decision,
        confidence,
        reasoning,
    })
}
