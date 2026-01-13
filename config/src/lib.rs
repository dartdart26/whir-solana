//! WHIR configuration constants and types.

use std::sync::Arc;

use ark_serialize::{CanonicalSerialize, Compress};
use spongefish_pow::blake3::Blake3PoW;
use whir_common::crypto::fields::Field64_2;
use whir_common::crypto::merkle_tree::blake3::{
    Blake3Compress, Blake3LeafHash, Blake3MerkleTreeParams,
};
use whir_common::crypto::merkle_tree::parameters::default_config;
use whir_common::ntt::RSDefault;
use whir_common::parameters::{
    default_max_pow, DeduplicationStrategy, FoldingFactor, MerkleProofStrategy,
    MultivariateParameters, ProtocolParameters, SoundnessType,
};
use whir_common::whir::parameters::WhirConfig;

/// The field type used for WHIR proofs.
pub type F = Field64_2;

/// Merkle tree configuration.
pub type MerkleConfig = Blake3MerkleTreeParams<F>;

/// Proof-of-work strategy.
pub type PowStrategy = Blake3PoW;

/// Number of variables in the multilinear polynomial.
pub const NUM_VARIABLES: usize = 6;

/// Security level in bits.
pub const SECURITY_LEVEL_BITS: usize = 100;

/// Starting log inverse rate for Reed-Solomon encoding.
pub const STARTING_LOG_INV_RATE: usize = 1;

/// Folding factor for the protocol.
pub const FOLDING_FACTOR: usize = 4;

/// Proof-of-work bits.
pub const POW_BITS: usize = default_max_pow(NUM_VARIABLES, STARTING_LOG_INV_RATE);

/// Domain separator for WHIR proofs (must match between prover and verifier).
pub const DOMAIN_SEPARATOR: &str = "whir-solana";

/// Returns the serialized size of a field element in bytes.
pub fn field_size_bytes() -> usize {
    F::default().serialized_size(Compress::Yes)
}

pub type WhirParams = WhirConfig<F, MerkleConfig, PowStrategy>;

pub fn create_whir_params(
    num_variables: usize,
    security_level: usize,
    pow_bits: usize,
    folding_factor: usize,
    starting_log_inv_rate: usize,
) -> WhirParams {
    // No need for a real RNG for parameter creation.
    let mut rng = ark_std::test_rng();

    let reed_solomon = Arc::new(RSDefault);
    let basefield_reed_solomon = Arc::new(RSDefault);

    let (leaf_hash_params, two_to_one_params) =
        default_config::<F, Blake3LeafHash<F>, Blake3Compress>(&mut rng);

    let mv_params = MultivariateParameters::<F>::new(num_variables);

    let protocol_params = ProtocolParameters::<MerkleConfig, PowStrategy> {
        initial_statement: true,
        security_level,
        pow_bits,
        folding_factor: FoldingFactor::ConstantFromSecondRound(folding_factor, folding_factor),
        leaf_hash_params,
        two_to_one_params,
        soundness_type: SoundnessType::ConjectureList,
        _pow_parameters: Default::default(),
        starting_log_inv_rate,
        batch_size: 1,
        deduplication_strategy: DeduplicationStrategy::Enabled,
        merkle_proof_strategy: MerkleProofStrategy::Compressed,
    };

    WhirConfig::new(
        reed_solomon,
        basefield_reed_solomon,
        mv_params,
        protocol_params,
    )
}
