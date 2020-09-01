FROM rust:slim

LABEL name="rust-musl-builder"
LABEL version="1.0.0"
LABEL repository="https://github.com/juankaram/rust-musl-action"
LABEL homepage="https://github.com/juankaram/rust-musl-action"
LABEL maintainer="Juan Karam"

LABEL com.github.actions.name="Rust MUSL Builder"
LABEL com.github.actions.description="Provides a Rust MUSL environment"
LABEL com.github.actions.icon="settings"
LABEL com.github.actions.color="orange"

RUN apt-get update && apt-get install -y zip build-essential musl-tools pkg-config libssl-dev

ENV BUILD_DIR=/build \
    OUTPUT_DIR=/output \
    RUST_BACKTRACE=1 \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    PREFIX=/toolchain \
    MUSL_VERSION=1.1.22 \
    OPENSSL_VERSION=1.1.0k \
    BUILD_TARGET=x86_64-unknown-linux-musl

RUN mkdir -p /usr/local/cargo/bin \
    && mkdir -p $BUILD_DIR \
    && mkdir -p $OUTPUT_DIR \
    && mkdir -p $PREFIX

WORKDIR $PREFIX

ADD http://www.musl-libc.org/releases/musl-$MUSL_VERSION.tar.gz .
RUN tar -xvzf musl-$MUSL_VERSION.tar.gz \
    && cd musl-$MUSL_VERSION \
    && ./configure --prefix=$PREFIX \
    && make install \
    && cd ..

ENV CC=$PREFIX/bin/musl-gcc \
    C_INCLUDE_PATH=$PREFIX/include/ \
    CPPFLAGS=-I$PREFIX/include \
    LDFLAGS=-L$PREFIX/lib

ADD https://www.openssl.org/source/openssl-$OPENSSL_VERSION.tar.gz .

RUN echo "Building OpenSSL" \
    && tar -xzf "openssl-$OPENSSL_VERSION.tar.gz" \
    && cd openssl-$OPENSSL_VERSION \
    && ./Configure no-async no-afalgeng no-shared no-zlib -fPIC --prefix=$PREFIX --openssldir=$PREFIX/ssl linux-x86_64 \
    && make depend \
    && make install

ENV OPENSSL_DIR=$PREFIX \
    OPENSSL_STATIC=true

WORKDIR $BUILD_DIR

RUN rustup self update && rustup update
RUN rustup target add $BUILD_TARGET
RUN rustup component add clippy-preview
RUN rustup component add rustfmt-preview
RUN cargo install cargo-release

COPY entrypoint.sh /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]