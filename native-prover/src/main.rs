//! WHIR Proof Generator CLI
//!
//! Generates WHIR proofs that can be verified by the Solana program.

use std::fs;
use whir_common::poly_utils::multilinear::MultilinearPoint;
use whir_proof_generator::{
    create_test_polynomial, generate_pcs_proof, verify_proof, ProofConfig, F,
};

fn main() -> anyhow::Result<()> {
    println!("WHIR Proof Generator for Solana");
    println!("================================");

    let config = ProofConfig::default();

    println!("Configuration:");
    println!("  - Number of variables: {}", config.num_variables);
    println!("  - Security level: {} bits", config.security_level);
    println!("  - PoW bits: {}", config.pow_bits);
    println!(
        "  - Starting log inverse rate: {}",
        config.starting_log_inv_rate
    );
    println!("  - Folding factor: {}", config.folding_factor);
    println!();

    println!("Creating test polynomial...");
    let polynomial = create_test_polynomial(config.num_variables);
    println!(
        "  - Polynomial has {} coefficients",
        1 << config.num_variables
    );
    println!();

    let eval_point = MultilinearPoint(
        (0..config.num_variables)
            .map(|i| F::from((i + 1) as u64))
            .collect(),
    );

    let expected_value = polynomial.evaluate_at_extension(&eval_point);

    println!("Generating proof...");
    let start = std::time::Instant::now();

    let proof = generate_pcs_proof(&config, &polynomial, &eval_point)?;

    let duration = start.elapsed();
    println!("  - Proof generated in {:?}", duration);
    println!("  - Proof size: {} bytes", proof.proof_bytes.len());
    println!();

    println!("Verifying proof natively...");

    verify_proof(&config, &proof, &eval_point, expected_value)?;

    // Save proof files to proof directory.
    fs::create_dir_all("proof").expect("Failed to create proof directory");

    fs::write("proof/proof.bin", &proof.proof_bytes).expect("Failed to write proof.bin");
    println!("Saved: proof/proof.bin");

    fs::write("proof/eval-point.bin", &proof.eval_point).expect("Failed to write eval-point.bin");
    println!("Saved: proof/eval-point.bin");

    fs::write("proof/eval-value.bin", &proof.eval_value).expect("Failed to write eval-value.bin");
    println!("Saved: proof/eval-value.bin");

    let metadata = serde_json::json!({
        "num_variables": proof.num_variables,
        "proof_size": proof.proof_bytes.len(),
        "eval_point_size": proof.eval_point.len(),
        "eval_value_size": proof.eval_value.len(),
        "config": {
            "security_level": config.security_level,
            "pow_bits": config.pow_bits,
            "starting_log_inv_rate": config.starting_log_inv_rate,
            "folding_factor": config.folding_factor,
        }
    });
    fs::write("proof/metadata.json", metadata.to_string()).expect("Failed to write metadata.json");
    println!("Saved: proof/metadata.json");
    Ok(())
}
