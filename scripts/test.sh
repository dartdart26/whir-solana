#!/bin/bash
set -e

echo "=== WHIR Solana Verifier Test Script ==="

# Change to project root directory.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

# Check if an existing validator is running and, if yes, stop.
if pgrep -f solana-test-validator >/dev/null; then
    echo "Solana test validator already running, please stop it and try again"
    exit 1
fi
# If not, remove any existing test-ledger directory.
rm -rf test-ledger

# Generate wallet keypair for tests, if it doesn't exist.
WALLET="$PROJECT_ROOT/.wallet/wallet-key.json"
if [ ! -f "$WALLET" ]; then
    echo "Generating test wallet keypair..."
    mkdir -p "$PROJECT_ROOT/.wallet"
    solana-keygen new --no-bip39-passphrase --force -o "$WALLET"
fi

# Generate program keypair, if it doesn't exist.
PROGRAM_KEYPAIR="$PROJECT_ROOT/program-keypair.json"
if [ ! -f "$PROGRAM_KEYPAIR" ]; then
    echo "Generating program keypair..."
    solana-keygen new --no-bip39-passphrase --force -o "$PROGRAM_KEYPAIR"
    echo "WARNING: New program keypair generated. Update declare_id! in lib.rs with:"
    solana-keygen pubkey "$PROGRAM_KEYPAIR"
fi

# Copy program keypair to target/deploy/ so Anchor uses consistent program ID.
mkdir -p target/deploy
cp "$PROGRAM_KEYPAIR" target/deploy/whir_verifier_solana-keypair.json

# Build and run prover to generate proof files.
echo "Building and running prover..."
cargo run -p native-prover --release

# Configure Solana CLI for localhost.
echo "Configuring Solana for localhost..."
solana config set --url localhost > /dev/null

# Start local validator in the background.
echo "Starting local validator..."
solana-test-validator --reset --quiet &
VALIDATOR_PID=$!

# Wait for validator to be ready.
echo "Waiting for validator to start..."
sleep 10

# Check if validator is running.
if ! kill -0 $VALIDATOR_PID 2>/dev/null; then
    echo "ERROR: Validator failed to start"
    exit 1
fi

# Airdrop SOL for testing.
echo "Airdropping SOL..."
solana airdrop 10 --keypair "$WALLET" > /dev/null

# Run tests.
echo "Running tests..."
echo ""
anchor test --skip-local-validator

# Cleanup.
echo ""
echo "Stopping validator..."
kill $VALIDATOR_PID 2>/dev/null || true

echo "Done!"
