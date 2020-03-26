
debug:
	GITUI_LOGGING=true cargo run --features=timing

test:
	cargo test --workspace -- --test-threads=1

install:
	cargo install --path "."

install-debug:
	cargo install --features=timing --path "."