import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { WhirVerifierSolana } from "../target/types/whir_verifier_solana";
import * as fs from "fs";
import { assert } from "chai";
import { Keypair } from "@solana/web3.js";

describe("whir_verifier_solana", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.WhirVerifierSolana as Program<WhirVerifierSolana>;
  const maxComputeUnits = 1_400_000;
  // Safe chunk size for transaction limits.
  const chunkSize = 800;
  // Account size for proof storage (increase for bigger proofs).
  const accountSize = 30 * 1024;

  interface ProofMetadata {
    num_variables: number;
    config: {
      security_level: number;
      pow_bits: number;
      folding_factor: number;
      starting_log_inv_rate: number;
    };
  }

  function loadProof(): {
    proof: Buffer;
    evalPoint: Buffer;
    evalValue: Buffer;
    metadata: ProofMetadata;
  } {
    const proof = fs.readFileSync("proof/proof.bin");
    const evalPoint = fs.readFileSync("proof/eval-point.bin");
    const evalValue = fs.readFileSync("proof/eval-value.bin");
    const metadata = JSON.parse(fs.readFileSync("proof/metadata.json", "utf-8"));
    return { proof, evalPoint, evalValue, metadata };
  }

  it("Verifies WHIR proof on-chain (multi-transaction)", async () => {
    console.log("\n=== WHIR PCS Verifier Test ===\n");

    const { proof, evalPoint, evalValue, metadata } = loadProof();

    console.log(`Proof size: ${proof.length} bytes`);
    console.log(`Evaluation point size: ${evalPoint.length} bytes`);
    console.log(`Evaluation value size: ${evalValue.length} bytes`);
    console.log(`Total: ${proof.length + evalPoint.length + evalValue.length} bytes`);
    console.log();

    // Create a new keypair for the proof data account.
    const proofDataKeypair = Keypair.generate();

    const rentExemption = await provider.connection.getMinimumBalanceForRentExemption(accountSize);

    // Step 0: Create the account.
    console.log(`0. Creating proof account (${accountSize} bytes, ${rentExemption / anchor.web3.LAMPORTS_PER_SOL} SOL rent)...`);
    const createAccountTx = await provider.sendAndConfirm(
      new anchor.web3.Transaction().add(
        anchor.web3.SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: proofDataKeypair.publicKey,
          space: accountSize,
          lamports: rentExemption,
          programId: program.programId,
        })
      ),
      [proofDataKeypair]
    );
    console.log(`   Create account transaction: ${createAccountTx}`);

    // Step 1: Initialize proof account
    console.log("1. Initializing proof account...");
    const initTx = await program.methods
      .initProof(Buffer.from(evalPoint), Buffer.from(evalValue))
      .accounts({
        proofData: proofDataKeypair.publicKey,
        payer: provider.wallet.publicKey,
      })
      .rpc();
    console.log(`   Init transaction: ${initTx}`);

    // Step 2: Upload proof in chunks.
    const numChunks = Math.ceil(proof.length / chunkSize);
    console.log(`2. Uploading proof in ${numChunks} chunk(s)...`);

    for (let i = 0; i < numChunks; i++) {
      const start = i * chunkSize;
      const end = Math.min(start + chunkSize, proof.length);
      const chunk = proof.subarray(start, end);

      const uploadTx = await program.methods
        .uploadChunk(Buffer.from(chunk))
        .accounts({
          proofData: proofDataKeypair.publicKey,
          payer: provider.wallet.publicKey,
        })
        .rpc();
      console.log(`   Chunk ${i + 1}/${numChunks} (${chunk.length} bytes): ${uploadTx}`);
    }

    // Step 3: Verify the proof.
    console.log("3. Verifying proof...");
    const modifyComputeUnits = anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
      units: maxComputeUnits,
    });

    const verifyTx = await program.methods
      .verify(
        metadata.num_variables,
        metadata.config.security_level,
        metadata.config.pow_bits,
        metadata.config.folding_factor,
        metadata.config.starting_log_inv_rate
      )
      .accounts({
        proofData: proofDataKeypair.publicKey,
      })
      .preInstructions([modifyComputeUnits])
      .rpc();
    console.log(`   Verify transaction: ${verifyTx}`);
    console.log("   Proof verified successfully on-chain!");

    // Step 4: Close the proof account to reclaim rent.
    console.log("4. Closing proof account...");
    const closeTx = await program.methods
      .closeProof()
      .accounts({
        proofData: proofDataKeypair.publicKey,
        payer: provider.wallet.publicKey,
      })
      .rpc();
    console.log(`   Close transaction: ${closeTx}`);
    console.log("\nAll steps completed successfully!");
  });

  it("Rejects invalid proof", async () => {
    console.log("\n=== Testing Invalid Proof Rejection ===\n");

    const { proof, evalPoint, evalValue, metadata } = loadProof();

    // Corrupt the proof.
    const corruptedProof = Buffer.from(proof);
    corruptedProof[100] ^= 0xff;

    const proofDataKeypair = Keypair.generate();

    const rentExemption = await provider.connection.getMinimumBalanceForRentExemption(accountSize);

    // Create account.
    await provider.sendAndConfirm(
      new anchor.web3.Transaction().add(
        anchor.web3.SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: proofDataKeypair.publicKey,
          space: accountSize,
          lamports: rentExemption,
          programId: program.programId,
        })
      ),
      [proofDataKeypair]
    );

    // Initialize proof account.
    await program.methods
      .initProof(Buffer.from(evalPoint), Buffer.from(evalValue))
      .accounts({
        proofData: proofDataKeypair.publicKey,
        payer: provider.wallet.publicKey,
      })
      .rpc();

    // Upload corrupted proof.
    const numChunks = Math.ceil(corruptedProof.length / chunkSize);
    for (let i = 0; i < numChunks; i++) {
      const start = i * chunkSize;
      const end = Math.min(start + chunkSize, corruptedProof.length);
      const chunk = corruptedProof.slice(start, end);

      await program.methods
        .uploadChunk(Buffer.from(chunk))
        .accounts({
          proofData: proofDataKeypair.publicKey,
          payer: provider.wallet.publicKey,
        })
        .rpc();
    }

    // Try to verify - should fail.
    const modifyComputeUnits = anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
      units: maxComputeUnits,
    });

    try {
      await program.methods
        .verify(
          metadata.num_variables,
          metadata.config.security_level,
          metadata.config.pow_bits,
          metadata.config.folding_factor,
          metadata.config.starting_log_inv_rate
        )
        .accounts({
          proofData: proofDataKeypair.publicKey,
        })
        .preInstructions([modifyComputeUnits])
        .rpc();

      assert.fail("Should have rejected invalid proof");
    } catch (error: any) {
      console.log(`Invalid proof correctly rejected, error = ${error}`);
      assert.include(error.toString(), "Error");
    }

    // Close proof.
    await program.methods
      .closeProof()
      .accounts({
        proofData: proofDataKeypair.publicKey,
        payer: provider.wallet.publicKey,
      })
      .rpc();
  });
});
