##
##make cargo-*
cargo-help:### 	cargo-help
	@awk 'BEGIN {FS = ":.*?###"} /^[a-zA-Z_-]+:.*?###/ {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

cargo-install:### 	cargo install --path .
#
	#@$(CARGO) install --path $(PWD)
	@$(CARGO) install --locked --path $(PWD)

cargo-i-gnostr-legit:cargo-install-gnostr-legit### 	cargo-i-gnostr-legit
cargo-install-gnostr-legit:
	@$(CARGO) install --bins $(QUIET) --path ./legit
cargo-bench:### 	cargo-bench
## $(CARGO) bench
	@$(CARGO) bench

cargo-examples:### 	cargo-examples
## $(CARGO) b --examples
	@$(CARGO) b --examples

cargo-report:### 	cargo-report
	$(CARGO) report future-incompatibilities --id 1

cargo-doc:### 	cargo-doc
	 $(CARGO) doc #--no-deps #--open

cargo-nightly-udeps:### 	cargo-nightly-udeps
	 $(CARGO) +nightly udeps

# vim: set noexpandtab:
# vim: set setfiletype make
