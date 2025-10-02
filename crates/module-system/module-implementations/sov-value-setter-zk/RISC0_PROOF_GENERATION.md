# RISC0 Proof Generation for Value-Setter-ZK

This guide explains how to generate real RISC0 proofs for the value-setter-zk module.

## Overview

The value-setter-zk module requires cryptographic proofs to validate value updates. This document covers:

1. Setting up the RISC0 toolchain
2. Building the guest program
3. Generating proofs off-chain
4. Creating transactions with proofs
5. Submitting to the rollup

## Architecture

```
┌─────────────────────┐
│   Off-Chain Host    │
│                     │
│  1. Input: value    │
│  2. Execute guest   │
│  3. Generate proof  │
└──────────┬──────────┘
           │
           │ Proof + Value
           │
           ▼
┌─────────────────────┐
│   Rollup (On-Chain) │
│                     │
│  1. Verify proof    │
│  2. Check value     │
│  3. Store if valid  │
└─────────────────────┘
```

## Prerequisites

### 1. Install RISC0 Toolchain

```bash
# Install rzup (RISC0 installer)
curl -L https://risczero.com/install | bash

# Install RISC0 components
rzup install

# Verify installation
cargo risczero --version
```

For detailed instructions, see: https://dev.risczero.com/api/zkvm/install

### 2. Build Dependencies

Make sure you have:
- Rust 1.77+ (check with `rustc --version`)
- Cargo
- 8GB+ RAM for proof generation
- Sufficient disk space (~5GB for RISC0 toolchain)

## Building the Guest Program

The guest program is the code that runs inside the RISC0 zkVM and generates the proof.

### Build the Guest Binary

```bash
cd crates/module-system/module-implementations/sov-value-setter-zk

# Build the guest program (this compiles to RISC0 zkVM target)
cargo build --release --features prove
```

This will:
1. Compile the guest program (`methods/guest/src/bin/value_setter.rs`)
2. Generate the method ID (circuit identifier)
3. Embed the ELF binary for the prover

**Note**: The first build may take 10-15 minutes as it compiles RISC0 dependencies.

## Generating Proofs

### Method 1: Using the Rust Tool

Generate a proof for a specific value:

```bash
cd crates/module-system/module-implementations/sov-value-setter-zk

# Generate proof (this will take several minutes)
cargo run --release --example risc0_proof_generator --features prove -- 42
```

**Output** (JSON):
```json
{
  "method_id": [12, 34, 56, ...],
  "proof": [... proof bytes ...]
}
```

**Time Estimates**:
- Development mode: 5-10 minutes per proof
- With GPU acceleration: 1-2 minutes per proof

### Method 2: Using the Shell Script

Generate a complete transaction JSON:

```bash
cd crates/module-system/module-implementations/sov-value-setter-zk/examples

# Generate transaction with RISC0 proof
./generate_risc0_transaction.sh 42 > tx.json

# With gas specification
./generate_risc0_transaction.sh 100 '[1000, 0]' > tx.json
```

## Customizing Validation Logic

Edit `methods/guest/src/bin/value_setter.rs` to add custom validation:

```rust
fn validate_value(value: u32) {
    // Your custom logic here
    
    // Example: only allow even numbers
    assert!(value % 2 == 0, "Value must be even");
    
    // Example: check range
    assert!(value >= 100 && value <= 1000, "Value out of range");
    
    // Example: verify computation
    // assert!(is_prime(value), "Value must be prime");
}
```

After changes, rebuild:

```bash
cargo build --release --features prove
```

## Submitting Transactions

### 1. Generate Transaction with Proof

```bash
cd crates/module-system/module-implementations/sov-value-setter-zk/examples
./generate_risc0_transaction.sh 42 > ~/tx_with_proof.json
```

### 2. Import to sov-cli

```bash
cd examples/demo-rollup

../../target/debug/sov-cli transactions import from-file value-setter-zk \
  --max-fee 100000000 \
  --path ~/tx_with_proof.json
```

### 3. Sign and Submit

```bash
../../target/debug/sov-cli node publish
```

### 4. Verify Transaction

```bash
# Get TX hash from previous command output
curl -sS http://127.0.0.1:12346/ledger/txs/<TX_HASH>/events | jq
```

Expected event:
```json
{
  "type": "event",
  "key": "ValueSetterZk/NewValue",
  "value": {
    "new_value": 42
  }
}
```

## Performance Optimization

### GPU Acceleration (Optional)

For faster proof generation, you can use GPU acceleration:

1. **Install CUDA** (NVIDIA GPUs):
   ```bash
   # Follow NVIDIA's CUDA installation guide
   # https://developer.nvidia.com/cuda-downloads
   ```

2. **Enable Metal** (Apple Silicon):
   ```bash
   # Metal is automatically used on macOS with Apple Silicon
   # No additional setup required
   ```

3. **Build with GPU support**:
   ```bash
   cargo build --release --features prove,risc0-zkvm/cuda
   ```

### Parallel Proof Generation

Generate multiple proofs in parallel:

```bash
# Generate proofs for values 1-10
for i in {1..10}; do
    (cargo run --release --example risc0_proof_generator --features prove -- $i > "proof_$i.json" 2>&1) &
done
wait
```

## Troubleshooting

### "Out of Memory" Errors

**Problem**: Proof generation runs out of RAM

**Solution**:
- Close other applications
- Increase system swap space
- Use a machine with more RAM (8GB minimum, 16GB+ recommended)

### Slow Proof Generation

**Problem**: Proofs take too long to generate

**Solutions**:
1. Use `--release` mode (10x faster than debug)
2. Enable GPU acceleration
3. Use a more powerful CPU
4. Consider using RISC0's Bonsai proving service (cloud proving)

### Guest Build Failures

**Problem**: Guest program fails to compile

**Solutions**:
1. Verify RISC0 toolchain installation: `rzup --version`
2. Update RISC0: `rzup update`
3. Clean build artifacts: `cargo clean`
4. Check Rust version: `rustc --version` (need 1.77+)

### Invalid Proof Errors

**Problem**: Generated proof fails verification

**Possible causes**:
1. Mismatch between guest code and verification method ID
2. Different RISC0 versions between prover and verifier
3. Proof corruption during serialization

**Solutions**:
1. Rebuild guest program: `cargo build --release --features prove`
2. Ensure consistent RISC0 versions
3. Regenerate the proof

## Advanced Topics

### Proof Caching

Cache generated proofs to avoid regeneration:

```bash
# Generate and cache
cargo run --release --example risc0_proof_generator --features prove -- 42 > proof_42.json

# Reuse cached proof
cat proof_42.json
```

### Batch Proving Service

For production deployments, consider implementing a proving service:

```
┌──────────┐    ┌──────────────┐    ┌─────────┐
│  Client  │───▶│ Prove Server │───▶│ Rollup  │
└──────────┘    └──────────────┘    └─────────┘
   Request         Generate           Submit
    Value           Proof              with
                                      Proof
```

### Integration with Bonsai

RISC0 Bonsai is a proving service that generates proofs in the cloud:

1. Sign up at https://bonsai.xyz
2. Get API credentials
3. Configure in your proving service
4. Submit proving jobs via API

## Testing

### Unit Tests

Test proof generation without verification:

```bash
cargo test --features prove -- --ignored
```

### Integration Tests

Test with the full rollup:

```bash
cd examples/demo-rollup
make test-value-setter-zk
```

## Production Checklist

Before deploying to production:

- [ ] Guest program validation logic reviewed and tested
- [ ] Proof generation tested with valid/invalid inputs
- [ ] Performance benchmarked for expected load
- [ ] Error handling implemented for proof generation failures
- [ ] Monitoring and alerting set up
- [ ] Backup proving infrastructure in place
- [ ] Security audit completed (if applicable)

## Resources

- [RISC0 Documentation](https://dev.risczero.com/)
- [RISC0 GitHub](https://github.com/risc0/risc0)
- [Sovereign SDK Documentation](https://github.com/Sovereign-Labs/sovereign-sdk)
- [Value-Setter-ZK Module](./README.md)

## Support

For issues:
1. Check the troubleshooting section above
2. Search existing GitHub issues
3. Ask in the Sovereign SDK Discord
4. Create a new GitHub issue with details

