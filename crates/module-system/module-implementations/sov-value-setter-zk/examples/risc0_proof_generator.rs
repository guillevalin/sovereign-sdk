//! RISC0 Proof Generator for value-setter-zk
//!
//! This tool generates real RISC0 proofs for value-setter-zk transactions.
//!
//! Usage:
//!   cargo run --release --example risc0_proof_generator --features prove -- <value>
//!
//! Example:
//!   cargo run --release --example risc0_proof_generator --features prove -- 42

use risc0_zkvm::{default_prover, ExecutorEnv, Journal, ProverOpts, Receipt, VerifierContext};
use serde::Serialize;
use sov_rollup_interface::zk::Proof;
use sov_value_setter_zk::ValueSetterZkProofPayload;
use value_setter_zk_methods::VALUE_SETTER_ELF;

/// Input structure for the guest program
#[derive(Serialize)]
struct ValueSetterInput {
    value: u32,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <value>", args[0]);
        eprintln!("Example: {} 42", args[0]);
        std::process::exit(1);
    }
    
    let value: u32 = args[1].parse().expect("Value must be a valid u32");
    
    eprintln!("Generating RISC0 proof for value: {}", value);
    eprintln!("This may take a while...");
    
    // Generate the proof
    match generate_proof(value) {
        Ok(proof_payload) => {
            // Output as JSON
            let json = serde_json::to_string_pretty(&proof_payload)
                .expect("Failed to serialize proof");
            println!("{}", json);
            
            eprintln!("\n✓ Proof generated successfully!");
            eprintln!("Method ID: {:?}", proof_payload.method_id);
            eprintln!("Proof size: {} bytes", proof_payload.proof.len());
        }
        Err(e) => {
            eprintln!("Error generating proof: {}", e);
            std::process::exit(1);
        }
    }
}

/// Generates a RISC0 proof for the given value
fn generate_proof(value: u32) -> anyhow::Result<ValueSetterZkProofPayload> {
    // Create input for the guest program
    let input = ValueSetterInput { value };
    
    // Set up the executor environment with the input
    let env = ExecutorEnv::builder()
        .write(&input)?
        .build()?;
    
    // Get the ELF binary for the guest program
    let elf = VALUE_SETTER_ELF;
    
    // Generate the proof using the default prover
    eprintln!("Running prover...");
    let prover = default_prover();
    
    // Use default STARK proofs (works on all architectures including ARM64/Apple Silicon)
    // Note: Groth16 proofs (ProverOpts::groth16()) only work on x86_64
    let prove_info = prover.prove_with_ctx(
        env,
        &VerifierContext::default(),
        elf,
        &ProverOpts::default(),
    )?;
    
    let receipt = prove_info.receipt;
    
    // Compute the method ID (image ID) from the ELF
    // This uniquely identifies the guest program
    let image_id = risc0_zkvm::compute_image_id(elf)?;
    let method_id_bytes: [u8; 32] = image_id.as_bytes().try_into()?;
    
    // Serialize the receipt as the proof
    let proof_bytes = bincode::serialize(&Proof::<Receipt, Option<Journal>>::Full(receipt))?;
    
    // Create the proof payload
    Ok(ValueSetterZkProofPayload {
        method_id: method_id_bytes,
        proof: proof_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sov_risc0_adapter::{Risc0MethodId, Risc0Verifier};
    use sov_rollup_interface::zk::{CodeCommitment as _, ZkVerifier};
    use sov_value_setter_zk::ValueSetterZkProofPublicOutput;

    #[test]
    #[ignore] // This test is slow, run with --ignored
    fn test_generate_and_verify_proof() {
        let value = 42;
        
        // Generate proof
        let payload = generate_proof(value).expect("Failed to generate proof");
        
        // Verify the proof
        let method_id = Risc0MethodId::decode(&payload.method_id)
            .expect("Failed to decode method ID");
        
        let output: ValueSetterZkProofPublicOutput = Risc0Verifier::verify(&payload.proof, &method_id)
            .expect("Proof verification failed");
        
        // Check that the output matches the input
        assert_eq!(output.new_value, value);
    }
}

