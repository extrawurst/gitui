
.PHONY: debug build-release release-linux-musl test clippy clippy-pedantic install install-debug

profile:
	cargo run --features=timing,pprof -- -l

debug:
	RUST_BACKTRACE=true cargo run --features=timing -- -l

build-release:
	cargo build --release

release-mac: build-release
	strip target/release/gitui
	mkdir -p release
	tar -C ./target/release/ -czvf ./release/gitui-mac.tar.gz ./gitui
	ls -lisah ./release/gitui-mac.tar.gz

release-win: build-release
	mkdir -p release
	tar -C ./target/release/ -czvf ./release/gitui-win.tar.gz ./gitui.exe
	cargo install cargo-wix
	cargo wix --no-build --nocapture --output ./release/gitui.msi
	ls -l ./release/gitui.msi 

release-linux-musl: build-linux-musl-release
	strip target/x86_64-unknown-linux-musl/release/gitui
	mkdir -p release
	tar -C ./target/x86_64-unknown-linux-musl/release/ -czvf ./release/gitui-linux-musl.tar.gz ./gitui

build-linux-musl-debug:
	cargo build --target=x86_64-unknown-linux-musl

build-linux-musl-release:
	cargo build --release --target=x86_64-unknown-linux-musl

test-linux-musl:
	cargo test --workspace --target=x86_64-unknown-linux-musl

test:
	cargo test --workspace

fmt:
	cargo fmt -- --check

clippy:
	touch src/main.rs
	cargo clean -p gitui -p asyncgit -p scopetime
	cargo clippy --workspace --all-features

clippy-nightly:
	touch src/main.rs
	cargo clean -p gitui -p asyncgit -p scopetime
	cargo +nightly clippy --all-features

clippy-pedantic:
	cargo clean -p gitui -p asyncgit -p scopetime
	cargo clippy --all-features -- -W clippy::pedantic

check: fmt clippy test

install:
	cargo install --path "." --offline

install-timing:
	cargo install --features=timing --path "." --offline