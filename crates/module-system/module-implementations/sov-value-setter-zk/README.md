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
