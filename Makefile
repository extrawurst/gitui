
debug:
	GITUI_LOGGING=true cargo run --features=timing

install:
	cargo install --path "."

install-debug:
	cargo install --features=timing --path "."