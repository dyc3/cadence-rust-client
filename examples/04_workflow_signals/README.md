# Example 04: Workflow Signals

## Overview

This example demonstrates workflow signal handling patterns. Signals allow external systems to communicate with running workflows, enabling dynamic updates, approvals, and interactive workflows.

## Features Demonstrated

- **Signal Reception**: Receiving and processing signals from external systems
- **Multiple Signal Types**: Handling different signals in a single workflow
- **Signal Channels**: Using async channels for signal handling
- **Signal-Driven State**: Updating workflow state based on signal data
- **Human-in-the-Loop**: Waiting for human approval via signals

## Key Concepts

### Signal Reception

Workflows receive signals through channels:

```rust
// Create a signal channel
let mut status_channel: SignalChannel = ctx.get_signal_channel("update_status");

// Wait for signal (blocking)
let signal_data = status_channel.recv().await;

if let Some(data) = signal_data {
    let signal: MySignal = serde_json::from_slice(&data)?;
    // Handle signal...
}
```

### Multiple Signal Types

Workflows can handle different signal types:

```rust
let mut status_channel = ctx.get_signal_channel("update_status");
let mut cancel_channel = ctx.get_signal_channel("cancel_order");
let mut add_item_channel = ctx.get_signal_channel("add_item");

// Use select! to handle any signal
loop {
    tokio::select! {
        data = status_channel.recv() => { /* handle status update */ }
        data = cancel_channel.recv() => { /* handle cancellation */ }
        data = add_item_channel.recv() => { /* handle add item */ }
    }
}
```

### Signal-Driven State Updates

Signals can update workflow state:

```rust
// Initialize state
let mut state = OrderState::new();

// Update state based on signals
match signal {
    AddItemSignal { item_id, quantity } => {
        state.items.push(OrderItem { item_id, quantity });
    }
    UpdateStatusSignal { new_status } => {
        state.status = new_status;
    }
}
```

### Human-in-the-Loop Pattern

Wait for human approval via signals:

```rust
pub async fn approval_workflow(ctx: &mut WorkflowContext) -> Result<ApprovalResult, WorkflowError> {
    let mut approval_channel = ctx.get_signal_channel("approval");
    
    // Wait for approval (blocking)
    let signal_data = approval_channel.recv().await;
    
    if let Some(data) = signal_data {
        let signal: ApprovalSignal = serde_json::from_slice(&data)?;
        return Ok(ApprovalResult {
            approved: signal.approved,
            approver: signal.approver,
        });
    }
    
    Err(WorkflowError::Generic("No approval received".to_string()))
}
```

## Workflows

### order_management_workflow

Multi-signal workflow for order management:

**Signals:**
- `add_item`: Add items to the order
- `update_status`: Change order status
- `cancel_order`: Cancel the order

**Behavior:**
- Waits for signals in a loop
- Updates state based on signals
- Sends notifications on state changes
- Ends when order is delivered or cancelled

### approval_workflow

Simple signal-receive pattern for approvals:

**Signals:**
- `approval`: Contains approver name and decision

**Behavior:**
- Waits for single approval signal
- Returns approval result
- Sends notification of decision

### config_update_workflow

Signal aggregation pattern:

**Signals:**
- `config_update`: Configuration key-value pairs

**Behavior:**
- Collects multiple config signals
- Aggregates them into a list
- Returns all collected configs

## Activities

### send_notification_activity

Sends notifications about state changes:

```rust
let notif_input = NotificationInput {
    recipient: "user@example.com".to_string(),
    message: "Your order has shipped".to_string(),
    notification_type: NotificationType::Email,
};
```

### update_status_activity

Updates entity status in database:

```rust
let update_input = StatusUpdateInput {
    entity_id: order_id,
    new_status: "Shipped".to_string(),
    updated_by: "admin".to_string(),
};
```

## Running the Example

```bash
# Run the main demonstration
cargo run -p workflow_signals

# Run all tests
cargo test -p workflow_signals
```

## Signal Types

### UpdateStatusSignal
```rust
pub struct UpdateStatusSignal {
    pub new_status: OrderStatus,
    pub updated_by: String,
    pub reason: Option<String>,
}
```

### AddItemSignal
```rust
pub struct AddItemSignal {
    pub item_id: String,
    pub quantity: u32,
    pub unit_price: f64,
}
```

### CancelOrderSignal
```rust
pub struct CancelOrderSignal {
    pub reason: String,
    pub cancelled_by: String,
}
```

### ApprovalSignal
```rust
pub struct ApprovalSignal {
    pub approver: String,
    pub approved: bool,
    pub comments: Option<String>,
}
```

### ConfigUpdateSignal
```rust
pub struct ConfigUpdateSignal {
    pub config_key: String,
    pub config_value: String,
}
```

## When to Use Signals

### Use Signals When:
- You need to update running workflows
- Human approval is required
- External events should affect workflow state
- Real-time updates are needed
- Workflow behavior should be dynamic

### Common Patterns:
- **Approval workflows**: Wait for human decisions
- **Order management**: Update orders with new items/status
- **Configuration**: Update workflow behavior dynamically
- **Notifications**: Trigger notifications from external events

## Related Examples

- Previous: [03_activity_advanced](../03_activity_advanced) - Heartbeats and cancellation
- Next: [05_workflow_external](../05_workflow_external) - External signals and cancellation
