_default:
    just --list

test-grpc-integration:
    cargo test --test grpc_integration -- --ignored --test-threads=1

test-ecommerce-saga:
    cargo test --test ecommerce_saga_integration -- --ignored --nocapture --test-threads=1

test-all-integration:
    cargo test -p cadence-client -- --ignored --test-threads=1
