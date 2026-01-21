//! WHIR Prover
//!
//! This library provides functionality to generate WHIR proofs that can be verified
//! by the Solana program.

use ark_ff::Field;
use ark_serialize::CanonicalSerialize;
use spongefish::{DomainSeparator, ProverState, VerifierState};
use whir_common::{
    poly_utils::{coeffs::CoefficientList, multilinear::MultilinearPoint},
    whir::{
        committer::{reader::CommitmentReader, writer::CommitmentWriter},
        domainsep::WhirDomainSeparator,
        statement::{Statement, Weights},
    },
};
use whir_config::WhirParams;
use whir_prover::Prover;
use whir_verifier::Verifier;

pub use whir_config::{MerkleConfig, PowStrategy, F, DOMAIN_SEPARATOR};

/// A serializable proof that can be sent to Solana.
#[derive(Clone)]
pub struct WhirProof {
    /// The serialized proof bytes.
    pub proof_bytes: Vec<u8>,
    /// The polynomial evaluation point.
    pub eval_point: Vec<u8>,
    /// The claimed evaluation value.
    pub eval_value: Vec<u8>,
    /// Number of variables in the polynomial.
    pub num_variables: usize,
}

/// Configuration for proof generation.
#[derive(Clone)]
pub struct ProofConfig {
    pub num_variables: usize,
    pub security_level: usize,
    pub pow_bits: usize,
    pub starting_log_inv_rate: usize,
    pub folding_factor: usize,
}

impl Default for ProofConfig {
    fn default() -> Self {
        Self {
            num_variables: whir_config::NUM_VARIABLES,
            security_level: whir_config::SECURITY_LEVEL_BITS,
            pow_bits: whir_config::POW_BITS,
            starting_log_inv_rate: whir_config::STARTING_LOG_INV_RATE,
            folding_factor: whir_config::FOLDING_FACTOR,
        }
    }
}

pub fn create_whir_params(config: &ProofConfig) -> WhirParams {
    whir_config::create_whir_params(
        config.num_variables,
        config.security_level,
        config.pow_bits,
        config.folding_factor,
        config.starting_log_inv_rate,
    )
}

/// Create a test polynomial with coefficients in the base prime field
pub fn create_test_polynomial(
    num_variables: usize,
) -> CoefficientList<<F as Field>::BasePrimeField> {
    let num_coeffs = 1 << num_variables;
    CoefficientList::new(
        (0..num_coeffs)
            .map(<F as Field>::BasePrimeField::from)
            .collect(),
    )
}

/// Generate a WHIR proof for PCS (Polynomial Commitment Scheme)
///
/// This generates a proof that the polynomial evaluates to a specific value at a given point.
pub fn generate_pcs_proof(
    config: &ProofConfig,
    polynomial: &CoefficientList<<F as Field>::BasePrimeField>,
    eval_point: &MultilinearPoint<F>,
) -> anyhow::Result<WhirProof> {
    let params = create_whir_params(config);

    // Create domain separator
    let domainsep = DomainSeparator::new(DOMAIN_SEPARATOR)
        .commit_statement(&params)
        .add_whir_proof(&params);

    let mut prover_state: ProverState = domainsep.to_prover_state();

    // Create commitment
    let committer = CommitmentWriter::new(params.clone());
    let witness = committer.commit(&mut prover_state, polynomial)?;

    // Create statement with evaluation constraint
    let mut statement = Statement::new(config.num_variables);

    // Compute expected evaluation
    let expected_value = polynomial.evaluate_at_extension(eval_point);

    let weights = Weights::evaluation(eval_point.clone());
    statement.add_constraint(weights, expected_value);

    // Generate proof
    let prover = Prover::new(params.clone());
    prover.prove(&mut prover_state, statement.clone(), witness)?;

    // Serialize the proof
    let proof_bytes = prover_state.narg_string().to_vec();

    // Serialize eval point
    let mut eval_point_bytes = Vec::new();
    for p in eval_point.0.iter() {
        p.serialize_compressed(&mut eval_point_bytes)?;
    }

    // Serialize evaluation value
    let mut eval_value_bytes = Vec::new();
    expected_value.serialize_compressed(&mut eval_value_bytes)?;

    Ok(WhirProof {
        proof_bytes,
        eval_point: eval_point_bytes,
        eval_value: eval_value_bytes,
        num_variables: config.num_variables,
    })
}

/// Verify a proof.
pub fn verify_proof(
    config: &ProofConfig,
    proof: &WhirProof,
    eval_point: &MultilinearPoint<F>,
    eval_value: F,
) -> anyhow::Result<()> {
    let params = create_whir_params(config);

    let domainsep = DomainSeparator::new(DOMAIN_SEPARATOR)
        .commit_statement(&params)
        .add_whir_proof(&params);

    // Reconstruct verifier state from proof.
    let mut verifier_state: VerifierState = domainsep.to_verifier_state(&proof.proof_bytes);

    // Parse commitment.
    let commitment_reader = CommitmentReader::new(&params);
    let parsed_commitment = commitment_reader.parse_commitment(&mut verifier_state)?;

    // Create statement.
    let mut statement = Statement::new(config.num_variables);
    statement.add_constraint(Weights::evaluation(eval_point.clone()), eval_value);

    // Verify.
    let verifier = Verifier::new(&params);
    verifier.verify(&mut verifier_state, parsed_commitment, statement)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_generation_and_verification() -> anyhow::Result<()> {
        let config = ProofConfig {
            num_variables: 6, // Small for testing
            security_level: 32,
            pow_bits: 5,
            starting_log_inv_rate: 1,
            folding_factor: 2,
        };

        // Create test polynomial.
        let poly = create_test_polynomial(config.num_variables);

        // Create evaluation point
        let eval_point = MultilinearPoint(
            (0..config.num_variables)
                .map(|i| F::from((i + 1) as u64))
                .collect(),
        );

        // Compute expected value.
        let expected_value = poly.evaluate_at_extension(&eval_point);

        // Generate proof.
        let proof =
            generate_pcs_proof(&config, &poly, &eval_point).expect("Failed to generate proof");

        println!("Proof size: {} bytes", proof.proof_bytes.len());

        // Verify locally.
        verify_proof(&config, &proof, &eval_point, expected_value)?;

        Ok(())
    }
}
