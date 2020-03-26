
debug:
	GITUI_LOGGING=true cargo run --features=timing

test:
	cargo test --workspace -- --test-threads=1

clippy:
	cargo clean
	cargo clippy --all-features

install:
	cargo install --path "."

install-debug:
	cargo install --features=timing --path "."