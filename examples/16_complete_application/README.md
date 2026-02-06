# Example 24: Complete Application

## Overview

This example demonstrates a production-ready complete e-commerce application using Cadence. It showcases real-world patterns for building scalable, fault-tolerant distributed systems.

## Features Demonstrated

- **User Registration**: Complete user onboarding with welcome notifications
- **Order Processing Saga**: Distributed transaction with compensation
- **Inventory Management**: Reservation patterns with expiration
- **Payment Processing**: Retry logic and failure handling
- **Multi-Channel Notifications**: Email, SMS, and push notifications
- **Order Fulfillment**: Shipping integration and tracking
- **Cancellation & Refunds**: Complete order lifecycle management

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  User           │────▶│  Registration    │────▶│  Welcome Email  │
│  Registration   │     │  Workflow        │     │  Notification   │
└─────────────────┘     └──────────────────┘     └─────────────────┘

┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Order          │────▶│  Order           │────▶│  Calculate      │
│  Request        │     │  Processing      │     │  Total          │
└─────────────────┘     │  Saga            │     └─────────────────┘
                        │                  │     ┌─────────────────┐
                        │  ┌──────────┐    │────▶│  Reserve        │
                        │  │Compensate│◀───┼─────│  Inventory      │
                        │  │on Failure│    │     └─────────────────┘
                        │  └──────────┘    │     ┌─────────────────┐
                        │                  │────▶│  Process        │
                        └──────────────────┘     │  Payment        │
                                                 └─────────────────┘
                                                          │
                               ┌──────────────────────────┼─────── Success
                               │                          │
                               ▼                          ▼
                        ┌──────────────────┐     ┌─────────────────┐
                        │  Release         │     │  Send           │
                        │  Inventory       │     │  Confirmation   │
                        └──────────────────┘     └─────────────────┘
                                                          │
                               ┌──────────────────────────┘
                               ▼
                        ┌──────────────────┐
                        │  Order           │
                        │  Fulfillment     │
                        └──────────────────┘
```

## Domain Model

### Core Entities

- **User**: Customer account with profile information
- **Product**: Catalog items with inventory tracking
- **Order**: Customer order with items and status
- **Payment**: Transaction details and status
- **InventoryReservation**: Reserved stock for orders
- **Shipment**: Shipping information and tracking

### Order Status Flow

```
Pending → Reserved → Paid → Processing → Shipped → Delivered
            │         │
            └─────────┴──▶ Cancelled
```

## Key Concepts

### Saga Pattern

The order processing uses the saga pattern for distributed transactions:

1. **Calculate Total**: Compute order amounts
2. **Reserve Inventory**: Hold stock for the order
3. **Process Payment**: Charge customer's payment method
4. **Send Confirmation**: Notify customer of success

If any step fails, compensation actions rollback previous steps:
- Release inventory reservations
- Refund processed payments
- Send failure notifications

### Compensation Strategy

```rust
match payment_result {
    Ok(payment) => {
        // Continue with order fulfillment
    }
    Err(e) => {
        // Compensate: release inventory
        release_inventory(reservation_id).await?;
        // Send failure notification
        notify_customer_of_failure().await?;
        return Err(e);
    }
}
```

### Retry Policy

Payment processing includes intelligent retry logic:

```rust
RetryPolicy {
    initial_interval: Duration::from_secs(1),
    backoff_coefficient: 2.0,
    maximum_interval: Duration::from_secs(30),
    maximum_attempts: 5,
    non_retryable_error_types: vec!["InvalidPaymentMethod"],
}
```

## Workflows

### user_registration_workflow
Complete user onboarding:
1. Create user account
2. Send welcome email
3. Return user profile

### order_processing_saga
Core order transaction:
1. Calculate totals
2. Reserve inventory
3. Process payment (with retry)
4. Send confirmation
5. Handle compensation on failure

### order_fulfillment_workflow
Post-payment fulfillment:
1. Create shipping label
2. Send shipping notification
3. Return tracking information

### order_cancellation_workflow
Order cancellation:
1. Release inventory reservation
2. Process refund
3. Send cancellation notification

## Running the Example

```bash
# Run the main demonstration
cargo run -p complete_application

# Run all tests
cargo test -p complete_application

# Run specific test with output
cargo test -p complete_application test_complete_order_lifecycle -- --nocapture
```

## Code Structure

```
src/
├── lib.rs           # Library exports
├── main.rs          # Example demonstration and tests
├── models/          # Domain models
│   └── mod.rs       # User, Order, Payment, etc.
├── activities/      # Business logic
│   └── mod.rs       # User, inventory, payment, shipping activities
└── workflows/       # Process orchestration
    └── mod.rs       # Registration, order, fulfillment workflows
```

## Production Considerations

### Database Integration

Replace mock data access with actual database calls:

```rust
pub async fn reserve_inventory_activity(
    ctx: &ActivityContext,
    order_id: String,
    items: Vec<OrderItem>,
) -> Result<InventoryReservation, ActivityError> {
    // Use actual database transaction
    let mut tx = db.begin_transaction().await?;
    
    for item in &items {
        let available = tx.check_stock(&item.product_id).await?;
        if available < item.quantity {
            return Err(ActivityError::Application("Insufficient stock".into()));
        }
        tx.reserve_stock(&item.product_id, item.quantity).await?;
    }
    
    tx.commit().await?;
    Ok(reservation)
}
```

### External Service Integration

Integrate with real payment providers:

```rust
pub async fn process_payment_activity(
    ctx: &ActivityContext,
    payment_info: PaymentInfo,
) -> Result<PaymentResult, ActivityError> {
    let client = PaymentProviderClient::new();
    
    let result = client
        .charge(&payment_info)
        .await
        .map_err(|e| ActivityError::Retryable(e.to_string()))?;
    
    Ok(result)
}
```

### Monitoring and Observability

Add comprehensive tracing and metrics:

```rust
pub async fn order_processing_saga(
    ctx: &mut WorkflowContext,
    input: OrderInput,
) -> Result<OrderOutput, WorkflowError> {
    let span = tracing::info_span!("order_processing", order_id = %input.order_id);
    let _enter = span.enter();
    
    metrics::counter!("orders_started").increment(1);
    
    let start = Instant::now();
    let result = process_order(ctx, input).await;
    let duration = start.elapsed();
    
    metrics::histogram!("order_processing_duration").record(duration.as_secs_f64());
    
    match &result {
        Ok(_) => metrics::counter!("orders_completed").increment(1),
        Err(_) => metrics::counter!("orders_failed").increment(1),
    }
    
    result
}
```

### Configuration Management

Externalize configuration for different environments:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub payment_retry_max_attempts: u32,
    pub inventory_reservation_ttl_minutes: u32,
    pub notification_channels: Vec<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        envy::from_env()
    }
}
```

### Graceful Shutdown

Handle shutdown signals properly:

```rust
async fn run_worker(config: WorkerConfig) {
    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        let _ = shutdown_tx.send(()).await;
    });
    
    let worker = Worker::new(config).await;
    
    tokio::select! {
        _ = worker.run() => {
            info!("Worker completed");
        }
        _ = shutdown_rx.recv() => {
            info!("Shutting down gracefully...");
            worker.shutdown().await;
        }
    }
}
```

## Testing Strategy

The example includes comprehensive tests:

1. **Unit Tests**: Individual activity testing
2. **Integration Tests**: Workflow end-to-end testing
3. **Saga Tests**: Compensation and failure scenarios
4. **Lifecycle Tests**: Complete order flow testing

## Related Examples

- Previous: [23_performance](../23_performance) - Performance optimization
- Next: [25_best_practices](../25_best_practices) - Idiomatic patterns
