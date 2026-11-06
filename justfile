core:
    cargo run -p epikodi-core
tv:
    cd apps/tv && flutter run -d linux
remote:
    cd apps/remote && flutter run
setup:
    ./scripts/setup-dev.sh
lint:
    cargo fmt --check && cargo clippy --workspace -- -D warnings && flutter analyze
test:
    cargo test --workspace && flutter test
demo:
    ./scripts/install-demo/install.sh
