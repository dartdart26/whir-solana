# WHIR PCS Solana Verifier

The WHIR PCS verifier as a Solana program. This project demonstrates how to generate WHIR proofs off-chain
using native Rust and verify them on-chain using a Solana program.

**WARNING**: THIS PROJECT IS A PROTOTYPE AND IS NOT READY FOR PRODUCTION!

WHIR is originally implemented here https://github.com/WizardOfMenlo/whir.

## Overview

This project includes:

1. **Native Prover** (`native-prover/`): A Rust CLI tool for generating WHIR proofs natively
2. **Solana Verifier Program** (`programs/whir-verifier-solana/`): An Anchor-based Solana program that verifies WHIR proofs on-chain
3. **Config** (`config/`): A crate for handling common configuration settings between prover and verifier

### WHIR Verifier Tweaks

We use a WHIR fork from https://github.com/dartdart26/whir/tree/separate-verifier-to-upstream to support running the verifier as a Solana program.

The following tweaks have been made in it:
1. Separate WHIR to prover, verifier and common crates. This way, the verifier doesn't use randomness/threading and can safely be used on Solana.
2. The `disable-hash-counter` feature disables a global counter variable that was producing a symbol name that is too long for Solana.
3. The `disable-ntt-cache` feature disables a global cache variable that was producing a symbol name that is too long for Solana.
4. The `small-stack` feature disables a manual loop unroll that was overflowing the limited Solana stack.

### Solana Verifier

The `whir-verifier-solana` program accepts a proof and WHIR parameters and verifies the proof. This makes it an universal program that can be called by separate
apps, with their own WHIR parameters and proof/polynomial sizes.

To support bigger proofs in terms of byte size, proof verification is split to the following steps:
1. **init_proof()**: initialize an account to store the proof in
2. **upload_chunk()**: upload a proof chunk, one at a time
3. **verify()**: verify the proof
4. **close_proof()**: close the proof account when done to reclaim rent.

Note that there can be multiple concurrent proofs as each one can be for a separate payer address.

## Limitations

As of now, the verifier tries to allocate more memory that allowed on Solana when using more than 6 variables and using a security of 100 bits. More work is needed on that front in the future.

## Dependencies

The project uses the WHIR fork from https://github.com/dartdart26/whir/tree/separate-verifier-to-upstream. This implementation makes possible compiling
and running the verifier as a Solana program.

## Prerequisites

- Rust
- Solana CLI tools
- Anchor Framework
- Node.js and Yarn

Please look at the `.devcontainer` directory for reference on what is needed to run.

## Usage

### Sample Proofs

The `sample-proof/` directory contains a pre-generated valid proof for testing. You can use it if you don't want to generate proofs yourself via the native prover.

### Run Tests

```bash
cd scripts
./test.sh
```

This will:
1. Start a Solana validator
2. Create a test polynomial
3. Generate a WHIR proof
4. Send the proof to the Solana program to verify it

## License

MIT
