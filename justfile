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

# Run gRPC integration tests
test-grpc-integration:
    cargo test -p crabdance --test grpc_integration -- --ignored --test-threads=1 --nocapture

# Run ecommerce saga integration test
test-ecommerce-saga:
    cargo test -p crabdance --test ecommerce_saga_integration -- --ignored --test-threads=1 --nocapture

# Run channels and spawn integration test
test-channels-spawn:
    cargo test -p crabdance --test channels_spawn_integration -- --ignored --test-threads=1 --nocapture

# Run onboarding reminder integration test
test-onboarding-reminder:
    cargo test -p crabdance --test onboarding_reminder_integration -- --ignored --test-threads=1 --nocapture

# Run payment confirmation integration test
test-payment-confirmation:
    cargo test -p crabdance --test payment_confirmation_integration -- --ignored --test-threads=1 --nocapture

# Run all integration tests
test-all-integration:
    cargo test -p crabdance --tests -- --ignored --test-threads=1 --nocapture

# Run the load test tool (pass args like: just load-test worker or just load-test client --duration 60 high-throughput --target-rate 100)
load-test *args:
    cargo run --bin load_test --release -- {{ args }}
