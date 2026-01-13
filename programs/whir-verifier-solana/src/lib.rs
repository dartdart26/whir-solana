//! WHIR PCS Verifier Solana Program
//!
//! It accepts a proof and WHIR parameters, making the program universal as it can,
//! theoretically, verify any WHIR proof.

use anchor_lang::prelude::*;
use ark_serialize::CanonicalDeserialize;
use spongefish::DomainSeparator;
use whir_common::{
    poly_utils::multilinear::MultilinearPoint,
    whir::{
        committer::reader::CommitmentReader,
        domainsep::WhirDomainSeparator,
        statement::{Statement, Weights},
    },
};
use whir_config::{field_size_bytes, F, DOMAIN_SEPARATOR};
use whir_verifier::Verifier;

declare_id!("AnycMJFRbi6gLYUtLH9YGVcE9F7PxnC1BijCWQMM3h9a");

#[program]
pub mod whir_verifier_solana {
    use whir_config::create_whir_params;

    use super::*;

    /// Initialize a proof account to store proof data across multiple transactions.
    pub fn init_proof(
        ctx: Context<InitProof>,
        eval_point_bytes: Vec<u8>,
        eval_value_bytes: Vec<u8>,
    ) -> Result<()> {
        let proof_data = &mut ctx.accounts.proof_data;
        proof_data.payer = ctx.accounts.payer.key();
        proof_data.proof = Vec::new();
        proof_data.eval_point = eval_point_bytes;
        proof_data.eval_value = eval_value_bytes;
        Ok(())
    }

    /// Upload a chunk of proof data to the proof account.
    pub fn upload_chunk(ctx: Context<UploadChunk>, chunk: Vec<u8>) -> Result<()> {
        ctx.accounts.proof_data.proof.extend_from_slice(&chunk);
        Ok(())
    }

    /// Verify the proof stored in the proof account.
    pub fn verify(
        ctx: Context<VerifyProof>,
        num_variables: u8,
        security_level: u8,
        pow_bits: u8,
        folding_factor: u8,
        starting_log_inv_rate: u8,
    ) -> Result<()> {
        let proof_data = &ctx.accounts.proof_data;
        let proof_bytes = proof_data.proof.as_slice();
        let eval_point_bytes = proof_data.eval_point.as_slice();
        let eval_value_bytes = proof_data.eval_value.as_slice();

        msg!("WHIR Verifier: Starting verification");
        msg!(
            "Config: num_vars={}, security={}, pow_bits={}",
            num_variables,
            security_level,
            pow_bits
        );

        let params = create_whir_params(
            num_variables as usize,
            security_level as usize,
            pow_bits as usize,
            folding_factor as usize,
            starting_log_inv_rate as usize,
        );

        let domainsep = DomainSeparator::new(DOMAIN_SEPARATOR)
            .commit_statement(&params)
            .add_whir_proof(&params);

        let mut verifier_state = domainsep.to_verifier_state(proof_bytes);

        let commitment_reader = CommitmentReader::new(&params);
        let parsed_commitment = commitment_reader
            .parse_commitment(&mut verifier_state)
            .map_err(|_| WhirError::CommitmentParseError)?;

        let eval_point = deserialize_eval_point(eval_point_bytes)?;

        let eval_value = F::deserialize_compressed(eval_value_bytes)
            .map_err(|_| WhirError::DeserializationError)?;

        let mut statement = Statement::new(num_variables as usize);
        statement.add_constraint(Weights::evaluation(eval_point), eval_value);

        let verifier = Verifier::new(&params);
        verifier
            .verify(&mut verifier_state, &parsed_commitment, &statement)
            .map_err(|_| WhirError::VerificationFailed)?;

        msg!("WHIR Verifier: Verification successful!");

        Ok(())
    }

    /// Close the proof account and reclaim rent.
    pub fn close_proof(_ctx: Context<CloseProof>) -> Result<()> {
        Ok(())
    }
}

/// Account to store proof data across multiple transactions.
#[account]
pub struct ProofData {
    pub payer: Pubkey,
    pub proof: Vec<u8>,
    pub eval_point: Vec<u8>,
    pub eval_value: Vec<u8>,
}

#[derive(Accounts)]
pub struct InitProof<'info> {
    #[account(zero)]
    pub proof_data: Account<'info, ProofData>,
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct UploadChunk<'info> {
    #[account(mut, has_one = payer)]
    pub proof_data: Account<'info, ProofData>,
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct VerifyProof<'info> {
    pub proof_data: Account<'info, ProofData>,
}

#[derive(Accounts)]
pub struct CloseProof<'info> {
    #[account(mut, close = payer, has_one = payer)]
    pub proof_data: Account<'info, ProofData>,
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[error_code]
pub enum WhirError {
    #[msg("Failed to parse commitment from proof")]
    CommitmentParseError,
    #[msg("Failed to deserialize field element")]
    DeserializationError,
    #[msg("Proof verification failed")]
    VerificationFailed,
}

fn deserialize_eval_point(bytes: &[u8]) -> Result<MultilinearPoint<F>> {
    let field_size = field_size_bytes();
    let mut points = Vec::new();
    for chunk in bytes.chunks_exact(field_size) {
        let value =
            F::deserialize_compressed(chunk).map_err(|_| WhirError::DeserializationError)?;
        points.push(value);
    }
    Ok(MultilinearPoint(points))
}
