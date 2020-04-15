
.PHONY: test

debug:
	GITUI_LOGGING=true cargo run --features=timing

build-release:
	cargo build --release

release-mac: build-release
	strip target/release/gitui
	mkdir -p release
	tar -C ./target/release/ -czvf ./release/gitui-mac.tar.gz ./gitui

release-linux: build-release
	strip target/release/gitui
	mkdir -p release
	tar -C ./target/release/ -czvf ./release/gitui-linux.tar.gz ./gitui

test:
	cargo test --workspace

clippy:
	cargo clean
	cargo clippy --all-features

clippy-pedantic:
	cargo clean
	cargo clippy --all-features -- -W clippy::pedantic

install:
	cargo install --path "."

install-debug:
	cargo install --features=timing --path "."