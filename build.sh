#!/bin/bash
(
{
# export CARGO_ENCODED_RUSTFLAGS="-L $PROGRAMFILES\PostgreSQL\14\lib"
# export RUSTFLAGS='-L /C/Program\ Files/PostgreSQL/14/lib'
export RUST_BACKTRACE=Full
cargo check
cargo build --release
}
)

