//! Workflow implementations for search attributes example.

use crate::activities::{
    OrderInfo, OrderStatus, OrderProcessingResult, InventoryResult,
};
use cadence_core::ActivityOptions;
use cadence_workflow::WorkflowContext;
use cadence_workflow::context::WorkflowError;
use std::time::Duration;
use tracing::info;
use serde::{Deserialize, Serialize};

/// Search attributes for order workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSearchAttributes {
    pub order_id: String,
    pub customer_id: String,
    pub status: OrderStatus,
    pub priority: String,
    pub region: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Workflow demonstrating search attributes for order processing
pub async fn order_processing_with_search_attrs_workflow(
    ctx: &mut WorkflowContext,
    order: OrderInfo,
) -> Result<OrderProcessingResult, WorkflowError> {
    let workflow_info = ctx.workflow_info();
    let _workflow_id = workflow_info.workflow_execution.workflow_id.clone();
    let start_time = chrono::Utc::now().timestamp();
    
    info!(
        "Starting order processing workflow {} for customer {}",
        order.order_id, order.customer_id
    );
    
    // Set initial search attributes
    let priority_str = match order.priority {
        crate::activities::OrderPriority::Low => "low",
        crate::activities::OrderPriority::Normal => "normal",
        crate::activities::OrderPriority::High => "high",
        crate::activities::OrderPriority::Critical => "critical",
    };
    
    // In a real implementation, these would be set via the Cadence API
    let search_attrs = OrderSearchAttributes {
        order_id: order.order_id.clone(),
        customer_id: order.customer_id.clone(),
        status: OrderStatus::Received,
        priority: priority_str.to_string(),
        region: order.region.clone(),
        created_at: start_time,
        updated_at: start_time,
    };
    
    info!("Initial search attributes: {:?}", search_attrs);
    
    // Step 1: Validate order
    let validation_result = ctx
        .execute_activity(
            "validate_order",
            Some(serde_json::to_vec(&order).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let inventory: InventoryResult = serde_json::from_slice(&validation_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse validation: {}", e)))?;
    
    if !inventory.all_available {
        info!(
            "Order {} cancelled due to unavailable items: {:?}",
            order.order_id, inventory.unavailable_items
        );
        
        let cancel_result = ctx
            .execute_activity(
                "cancel_order",
                Some(serde_json::to_vec(&(order.order_id.clone(), "Items unavailable".to_string())).unwrap()),
                ActivityOptions::default(),
            )
            .await?;
        
        let result: OrderProcessingResult = serde_json::from_slice(&cancel_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse cancel result: {}", e)))?;
        
        return Ok(result);
    }
    
    info!("Order {} validated, updating search attributes", order.order_id);
    
    // Step 2: Process order
    let processing_result = ctx
        .execute_activity(
            "process_order",
            Some(serde_json::to_vec(&order).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let processed: OrderProcessingResult = serde_json::from_slice(&processing_result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse processing result: {}", e)))?;
    
    info!(
        "Order {} processed in {}ms, updating search attributes",
        order.order_id, processed.processing_time_ms
    );
    
    // Step 3: Ship order (for non-critical priority)
    if !matches!(order.priority, crate::activities::OrderPriority::Critical) {
        ctx.sleep(Duration::from_millis(100)).await;
        
        let ship_result = ctx
            .execute_activity(
                "ship_order",
                Some(serde_json::to_vec(&order.order_id).unwrap()),
                ActivityOptions::default(),
            )
            .await?;
        
        let shipped: OrderProcessingResult = serde_json::from_slice(&ship_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse ship result: {}", e)))?;
        
        info!("Order {} shipped, updating search attributes to shipped", order.order_id);
        
        return Ok(shipped);
    }
    
    info!("Order {} workflow completed with status: {:?}", order.order_id, processed.status);
    
    Ok(processed)
}

/// Workflow demonstrating priority-based search attributes
pub async fn priority_order_workflow(
    ctx: &mut WorkflowContext,
    order: OrderInfo,
) -> Result<OrderProcessingResult, WorkflowError> {
    let _workflow_id = ctx.workflow_info().workflow_execution.workflow_id.clone();
    
    info!(
        "Starting priority order workflow {} with {:?} priority",
        order.order_id, order.priority
    );
    
    // Set priority-based search attributes
    let priority_level = match order.priority {
        crate::activities::OrderPriority::Critical => 4,
        crate::activities::OrderPriority::High => 3,
        crate::activities::OrderPriority::Normal => 2,
        crate::activities::OrderPriority::Low => 1,
    };
    
    info!("Setting priority search attribute to level {}", priority_level);
    
    // Process based on priority
    let timeout = match order.priority {
        crate::activities::OrderPriority::Critical => Duration::from_secs(5),
        crate::activities::OrderPriority::High => Duration::from_secs(10),
        crate::activities::OrderPriority::Normal => Duration::from_secs(30),
        crate::activities::OrderPriority::Low => Duration::from_secs(60),
    };
    
    let activity_options = ActivityOptions {
        task_list: ctx.workflow_info().task_list.clone(),
        start_to_close_timeout: timeout,
        ..Default::default()
    };
    
    let result = ctx
        .execute_activity(
            "process_order",
            Some(serde_json::to_vec(&order).unwrap()),
            activity_options,
        )
        .await?;
    
    let processed: OrderProcessingResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    info!(
        "Priority order workflow completed for {} in {}ms",
        order.order_id, processed.processing_time_ms
    );
    
    Ok(processed)
}

/// Workflow demonstrating region-based search attributes
pub async fn regional_order_workflow(
    ctx: &mut WorkflowContext,
    order: OrderInfo,
) -> Result<OrderProcessingResult, WorkflowError> {
    info!(
        "Starting regional order workflow {} for region: {}",
        order.order_id, order.region
    );
    
    // Set region search attribute
    info!("Setting region search attribute to: {}", order.region);
    
    // Validate first
    let validation = ctx
        .execute_activity(
            "validate_order",
            Some(serde_json::to_vec(&order).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let inventory: InventoryResult = serde_json::from_slice(&validation)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse validation: {}", e)))?;
    
    if !inventory.all_available {
        let cancel_result = ctx
            .execute_activity(
                "cancel_order",
                Some(serde_json::to_vec(&(order.order_id.clone(), "Items unavailable".to_string())).unwrap()),
                ActivityOptions::default(),
            )
            .await?;
        
        let result: OrderProcessingResult = serde_json::from_slice(&cancel_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse cancel result: {}", e)))?;
        
        return Ok(result);
    }
    
    // Process order
    let result = ctx
        .execute_activity(
            "process_order",
            Some(serde_json::to_vec(&order).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let processed: OrderProcessingResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    // Update status
    let _ = ctx
        .execute_activity(
            "update_order_status",
            Some(serde_json::to_vec(&(order.order_id.clone(), OrderStatus::Processing)).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    info!(
        "Regional order workflow completed for {} in region {}",
        order.order_id, order.region
    );
    
    Ok(processed)
}

/// Query result for searching by attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderQueryResult {
    pub order_id: String,
    pub status: OrderStatus,
    pub priority: String,
    pub region: String,
    pub customer_id: String,
}

/// Workflow with query handler for searching
pub async fn searchable_order_workflow(
    ctx: &mut WorkflowContext,
    order: OrderInfo,
) -> Result<OrderProcessingResult, WorkflowError> {
    info!("Starting searchable order workflow {}", order.order_id);
    
    let _order_id = order.order_id.clone();
    let _customer_id = order.customer_id.clone();
    let _priority = format!("{:?}", order.priority);
    let _region = order.region.clone();
    let mut _current_status = OrderStatus::Received;
    
    // Process the order
    let validation = ctx
        .execute_activity(
            "validate_order",
            Some(serde_json::to_vec(&order).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let inventory: InventoryResult = serde_json::from_slice(&validation)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse validation: {}", e)))?;
    
    if !inventory.all_available {
        _current_status = OrderStatus::Cancelled;
        let cancel_result = ctx
            .execute_activity(
                "cancel_order",
                Some(serde_json::to_vec(&(order.order_id.clone(), "Items unavailable".to_string())).unwrap()),
                ActivityOptions::default(),
            )
            .await?;
        
        let result: OrderProcessingResult = serde_json::from_slice(&cancel_result)
            .map_err(|e| WorkflowError::Generic(format!("Failed to parse cancel result: {}", e)))?;
        
        return Ok(result);
    }
    
    _current_status = OrderStatus::Validated;
    
    let result = ctx
        .execute_activity(
            "process_order",
            Some(serde_json::to_vec(&order).unwrap()),
            ActivityOptions::default(),
        )
        .await?;
    
    let processed: OrderProcessingResult = serde_json::from_slice(&result)
        .map_err(|e| WorkflowError::Generic(format!("Failed to parse result: {}", e)))?;
    
    _current_status = processed.status.clone();
    
    info!("Searchable order workflow completed");
    
    Ok(processed)
}
