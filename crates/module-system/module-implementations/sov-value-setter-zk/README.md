# `sov-value-setter-zk`

A variant of the `sov-value-setter` module that requires a valid [RISC0](https://www.risczero.com/) proof for
all value updates. Transactions must include both the value to store and the proof payload so the module can
verify the proof inside the rollup.

## Behavior

- Stores an unsigned 32-bit integer in state and emits an event whenever the value changes.
- Keeps a copy of the most recent proof (method id, raw proof bytes, attested value) for auditing.
- Only the configured admin address may submit transactions that update the value.
- The module verifies the proof using the RISC0 verifier available in the Sovereign SDK workspace.
- If the proof does not verify or the public output mismatches the requested value, the transaction is reverted.

Proof generation is performed off-chain. Each transaction must include the proof bytes and the method id that
produced them so the verifier can authenticate execution.

## Quick Start

### For Development (Mock Proofs)

For testing and development, use mock proofs:

```bash
# Generate a mock proof
cargo run --example proof_generator --features native -- 42

# Generate transaction with mock proof
cd examples
./generate_transaction.sh 42 > tx.json
```

⚠️ **Mock proofs work only in test environments and will be rejected in production.**

### For Production (Real RISC0 Proofs)

For production deployments, generate real RISC0 proofs:

```bash
# 1. Install RISC0 toolchain (one-time setup)
curl -L https://risczero.com/install | bash
rzup install

# 2. Build the guest program
cargo build --release --features prove

# 3. Generate a real RISC0 proof (takes several minutes)
cargo run --release --example risc0_proof_generator --features prove -- 42

# Or use the helper script
cd examples
./generate_risc0_transaction.sh 42 > tx.json
```

## Documentation

- **[RISC0 Proof Generation Guide](./RISC0_PROOF_GENERATION.md)** - Complete guide for generating real proofs
- **[Transaction Examples](../../../../examples/test-data/requests/VALUE_SETTER_ZK_EXAMPLES.md)** - Usage examples and tutorials

## Proof Generation

### Guest Program

The RISC0 guest program (`methods/guest/src/bin/value_setter.rs`) validates values:

```rust
fn validate_value(value: u32) {
    assert!(value > 0, "Value must be greater than zero");
    assert!(value < 1_000_000, "Value must be less than 1,000,000");
    // Add your custom validation logic here
}
```

### Customizing Validation

Edit the guest program to add custom business logic:
- Range checks
- Mathematical constraints
- Cryptographic validations
- Rate limiting

After changes, rebuild: `cargo build --release --features prove`

## Tools Overview

| Tool | Purpose | Speed | Production Ready |
|------|---------|-------|------------------|
| `proof_generator` | Mock proofs for testing | Instant | ❌ No |
| `generate_transaction.sh` | Mock transaction JSON | Instant | ❌ No |
| `risc0_proof_generator` | Real RISC0 proofs | Minutes | ✅ Yes |
| `generate_risc0_transaction.sh` | Real proof transaction | Minutes | ✅ Yes |
