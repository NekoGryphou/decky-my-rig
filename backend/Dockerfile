FROM rust:1.98.1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e

RUN apt-get update \
    && apt-get install --yes --no-install-recommends clang \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add clippy rustfmt \
    && rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
