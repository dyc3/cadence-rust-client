_default:
    just --list

# Regenerate protobuf code from .proto files
gen-proto:
    cargo build -p crabdance_proto

# Determinism / replay gate: run the replay harness and the in-memory driver tests.
# A failure here means a workflow no longer replays its recorded history
# deterministically (the emitted command sequence diverged).
replay-gate:
    cargo test -p crabdance_testsuite --lib replay::
    cargo test -p crabdance_workflow --lib driver::

# Run every integration test (requires live services: a Cadence server, and
# Postgres for the sqlx test). Gated behind the `integration` feature.
integration:
    cargo test -p crabdance_client --features integration -- --test-threads=1
    cargo test -p crabdance --features integration --tests -- --test-threads=1 --nocapture

# Run gRPC integration tests
test-grpc-integration:
    cargo test -p crabdance --features integration --test grpc_integration -- --test-threads=1 --nocapture

# Run ecommerce saga integration test
test-ecommerce-saga:
    cargo test -p crabdance --features integration --test ecommerce_saga_integration -- --test-threads=1 --nocapture

# Run channels and spawn integration test
test-channels-spawn:
    cargo test -p crabdance --features integration --test channels_spawn_integration -- --test-threads=1 --nocapture

# Run onboarding reminder integration test
test-onboarding-reminder:
    cargo test -p crabdance --features integration --test onboarding_reminder_integration -- --test-threads=1 --nocapture

# Run payment confirmation integration test
test-payment-confirmation:
    cargo test -p crabdance --features integration --test payment_confirmation_integration -- --test-threads=1 --nocapture

# Run all integration tests (alias for `integration`)
test-all-integration: integration

# Run the load test tool (pass args like: just load-test worker or just load-test client --duration 60 high-throughput --target-rate 100)
load-test *args:
    cargo run --bin load_test --release -- {{ args }}
