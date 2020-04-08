
.PHONY: test

debug:
	GITUI_LOGGING=true cargo run --features=timing

build-release:
	cargo build --release
	strip target/release/gitui
	ls -lisah target/release/gitui
	tar -C ./target/release/ -czvf ./target/gitui-mac.tar.gz ./gitui
	ls -lisah ./target/gitui-mac.tar.gz
	shasum -a 256 ./target/gitui-mac.tar.gz | awk '{printf $1}'

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