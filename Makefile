
.PHONY: debug build-release release-linux-musl test clippy clippy-pedantic install install-debug

debug:
	cargo run --features=timing -- -l

build-release:
	cargo build --release

release-mac: build-release
	strip target/release/gitui
	mkdir -p release
	tar -C ./target/release/ -czvf ./release/gitui-mac.tar.gz ./gitui

release-win: build-release
	mkdir -p release
	tar -C ./target/release/ -czvf ./release/gitui-win.tar.gz ./gitui.exe

release-linux-musl: 
	cargo build --release --target=x86_64-unknown-linux-musl
	strip target/x86_64-unknown-linux-musl/release/gitui
	mkdir -p release
	tar -C ./target/x86_64-unknown-linux-musl/release/ -czvf ./release/gitui-linux-musl.tar.gz ./gitui

test:
	cargo test --workspace

fmt:
	cargo fmt -- --check

clippy:
	touch src/main.rs
	cargo clean -p gitui -p asyncgit -p scopetime
	cargo clippy --all-features

clippy-pedantic:
	cargo clean -p gitui -p asyncgit -p scopetime
	cargo clippy --all-features -- -W clippy::pedantic

check: fmt clippy

install:
	cargo install --path "."

install-debug:
	cargo install --features=timing --path "." --offline