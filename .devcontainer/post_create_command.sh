#!/bin/bash

set -euo pipefail

# Package manager dependencies.
sudo apt update
sudo apt install -y protobuf-compiler build-essential libssl-dev libudev-dev llvm libclang-dev pkg-config openssl vim

# Install the Rust toolchain.
RUST_VERSION=1.92.0
rustup toolchain install $RUST_VERSION
rustup component add --toolchain $RUST_VERSION rustfmt
rustup component add --toolchain $RUST_VERSION clippy

# Solana CLI
sh -c "$(curl -sSfL --proto '=https' --tlsv1.2 https://release.anza.xyz/stable/install)"
echo 'export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"' >> ~/.bashrc
agave-install update

# AVM and Anchor
cargo install --git https://github.com/solana-foundation/anchor avm --force
avm install latest
avm use latest

# Surfproof
curl -sSfL --proto '=https' --tlsv1.2 https://run.surfpool.run/ | bash
