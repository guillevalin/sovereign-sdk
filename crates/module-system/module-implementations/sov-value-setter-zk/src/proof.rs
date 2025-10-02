use std::sync::RwLock;

use borsh::{BorshDeserialize, BorshSerialize};
use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sov_modules_api::macros::{serialize, UniversalWallet};
#[cfg(not(target_os = "zkvm"))]
use sov_risc0_adapter::{Risc0MethodId, Risc0Verifier};
#[cfg(not(target_os = "zkvm"))]
use sov_rollup_interface::zk::{CodeCommitment, ZkVerifier};
use thiserror::Error;

/// Payload containing the data required to verify a RISC0 proof.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub struct ValueSetterZkProofPayload {
    /// The 32-byte method identifier for the circuit.
    pub method_id: [u8; 32],
    /// Proof bytes produced by the prover.
    pub proof: Vec<u8>,
}

/// Public output produced by verifying the proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ValueSetterZkProofPublicOutput {
    /// The new value authorized by the proof.
    pub new_value: u32,
}

/// Last verified proof stored on-chain.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    BorshSerialize,
    BorshDeserialize,
)]
#[serde(rename_all = "snake_case")]
pub struct StoredProof {
    /// The method identifier that was verified.
    pub method_id: [u8; 32],
    /// The serialized proof bytes.
    pub proof: Vec<u8>,
    /// The value attested by the proof.
    pub new_value: u32,
}

impl StoredProof {
    /// Creates a stored proof record from a verified payload.
    pub fn from_verified_payload(
        payload: ValueSetterZkProofPayload,
        output: ValueSetterZkProofPublicOutput,
    ) -> Self {
        Self {
            method_id: payload.method_id,
            proof: payload.proof,
            new_value: output.new_value,
        }
    }
}

/// Errors returned by the proof verifier.
#[derive(Debug, Error)]
pub enum ProofVerificationError {
    /// Proof verification is unavailable on this compilation target.
    #[cfg(target_os = "zkvm")]
    #[error("RISC0 proof verification is not available on this target")]
    UnsupportedTarget,
    /// Proof verification failed.
    #[cfg(not(target_os = "zkvm"))]
    #[error("RISC0 verifier returned an error: {reason}")]
    VerificationFailed {
        /// Message returned by the verifier.
        reason: String,
    },
}

type ProofVerifier = dyn Fn(&ValueSetterZkProofPayload) -> Result<ValueSetterZkProofPublicOutput, ProofVerificationError>
    + Send
    + Sync
    + 'static;

type ProofVerifierBox = Box<ProofVerifier>;

static PROOF_VERIFIER: Lazy<RwLock<ProofVerifierBox>> =
    Lazy::new(|| RwLock::new(Box::new(default_risc0_verify)));

/// Verifies the proof using the configured verifier implementation.
pub fn verify_proof(
    payload: &ValueSetterZkProofPayload,
) -> Result<ValueSetterZkProofPublicOutput, ProofVerificationError> {
    let guard = PROOF_VERIFIER.read().expect("proof verifier lock poisoned");
    (&**guard)(payload)
}

fn default_risc0_verify(
    payload: &ValueSetterZkProofPayload,
) -> Result<ValueSetterZkProofPublicOutput, ProofVerificationError> {
    #[cfg(target_os = "zkvm")]
    {
        let _ = payload;
        Err(ProofVerificationError::UnsupportedTarget)
    }
    #[cfg(not(target_os = "zkvm"))]
    {
        let method_id =
            <Risc0MethodId as CodeCommitment>::decode(&payload.method_id).map_err(|err| {
                ProofVerificationError::VerificationFailed {
                    reason: err.to_string(),
                }
            })?;
        Risc0Verifier::verify::<ValueSetterZkProofPublicOutput>(&payload.proof, &method_id).map_err(
            |err| ProofVerificationError::VerificationFailed {
                reason: err.to_string(),
            },
        )
    }
}

#[cfg(test)]
pub struct MockProofGuard {
    previous: Option<ProofVerifierBox>,
}

#[cfg(test)]
impl Drop for MockProofGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            let mut guard = PROOF_VERIFIER
                .write()
                .expect("proof verifier lock poisoned");
            *guard = previous;
        }
    }
}

#[cfg(test)]
pub fn install_mock_proof_verifier<F>(mock: F) -> MockProofGuard
where
    F: Fn(
            &ValueSetterZkProofPayload,
        ) -> Result<ValueSetterZkProofPublicOutput, ProofVerificationError>
        + Send
        + Sync
        + 'static,
{
    let mut guard = PROOF_VERIFIER
        .write()
        .expect("proof verifier lock poisoned");
    let previous = std::mem::replace(&mut *guard, Box::new(mock));
    MockProofGuard {
        previous: Some(previous),
    }
}
