//! Generated RISC0 method IDs and ELF binaries for value-setter-zk guest programs.

// Include the generated methods module when building
#[cfg(not(feature = "skip-guest-build"))]
include!(concat!(env!("OUT_DIR"), "/methods.rs"));

// Provide stub implementations when skipping guest build
#[cfg(feature = "skip-guest-build")]
pub const VALUE_SETTER_ELF: &[u8] = &[];
#[cfg(feature = "skip-guest-build")]
pub const VALUE_SETTER_ID: [u32; 8] = [0; 8];

