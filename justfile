_default:
    just --list

test-grpc-integration:
    cargo test --test grpc_integration -- --ignored --test-threads=1

test-ecommerce-saga:
    cargo test --test ecommerce_saga_integration -- --ignored --nocapture --test-threads=1

test-all-integration:
    cargo test -p uber_cadence_client -- --ignored --test-threads=1

# Run the load test tool (pass args like: just load-test worker or just load-test client --duration 60 high-throughput --target-rate 100)
load-test *args:
    cargo run --bin load_test --release -- {{ args }}
