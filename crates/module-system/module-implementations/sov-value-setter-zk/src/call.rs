use std::fmt::Debug;

use anyhow::Result;
use schemars::JsonSchema;
use sov_modules_api::macros::{serialize, UniversalWallet};
use sov_modules_api::{Context, EventEmitter, Gas, Spec, TxState};
use strum::{EnumDiscriminants, EnumIs, VariantArray};
use thiserror::Error;

use super::ValueSetterZk;
use crate::event::Event;
use crate::proof::{verify_proof, StoredProof, ValueSetterZkProofPayload};

/// This enumeration represents the available call messages for interacting with the `sov-value-setter-zk` module.
#[derive(Debug, PartialEq, Eq, Clone, JsonSchema, EnumDiscriminants, EnumIs, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[schemars(bound = "S::Gas: ::schemars::JsonSchema", rename = "CallMessage")]
#[strum_discriminants(derive(VariantArray, EnumIs))]
#[serde(rename_all = "snake_case")]
pub enum CallMessage<S: Spec> {
    /// Single value to set.
    SetValue {
        /// Singe new value.
        value: u32,
        /// Gas to charge. Don't charge gas if None.
        gas: Option<S::Gas>,
        /// Proof attesting to the validity of the new value.
        proof: ValueSetterZkProofPayload,
    },
    /// Many values to set.
    SetManyValues(
        /// Many new values.
        Vec<u8>,
    ),
    /// Assert the visible slot number is as expected.
    AssertVisibleSlotNumber {
        /// The expected visible slot number.
        expected_visible_slot_number: u64,
    },
    /// SetValue and sleep for a while.
    SetValueAndSleep {
        /// Singe new value.
        value: u32,
        /// The number of milliseconds to sleep.
        sleep_millis: u64,
        /// Proof attesting to the validity of the new value.
        proof: ValueSetterZkProofPayload,
    },
    /// Trigger a panic.
    Panic,
}

/// Example of a custom error.
#[derive(Debug, Error)]
pub enum SetValueError<S: Spec> {
    /// Value tried to be set by a user that wasn't admin.
    #[error(
        "Only admin can change the value. The expected admin is {admin}, but the sender is {sender}"
    )]
    WrongSender {
        /// The expected admin.
        admin: S::Address,
        /// The sender.
        sender: S::Address,
    },
    /// Proof provided by the caller failed verification.
    #[error("RISC0 proof verification failed: {reason}")]
    InvalidProof {
        /// Reason returned by the verifier.
        reason: String,
    },
    /// Proof output does not match the requested value.
    #[error("RISC0 proof output {proof_value} does not match requested value {requested_value}")]
    ProofValueMismatch {
        /// Value encoded in the verified proof output.
        proof_value: u32,
        /// Value requested by the caller.
        requested_value: u32,
    },
}

impl<S: Spec> ValueSetterZk<S> {
    /// Sets `value` field to the `new_value`, only admin is authorized to call this method.
    pub(crate) fn set_value(
        &mut self,
        new_value: u32,
        gas: Option<S::Gas>,
        proof: ValueSetterZkProofPayload,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<()> {
        let gas = gas.unwrap_or(<S::Gas as Gas>::zero());
        state.charge_gas(&gas)?;
        // If admin is not then early return:
        let admin = self.admin.get_or_err(state)??;

        if &admin != context.sender() {
            // Here we use a custom error type.
            Err(SetValueError::WrongSender::<S> {
                admin,
                sender: context.sender().clone(),
            })?;
        }

        let proof_output =
            verify_proof(&proof).map_err(|err| SetValueError::<S>::InvalidProof {
                reason: err.to_string(),
            })?;

        if proof_output.new_value != new_value {
            Err(SetValueError::<S>::ProofValueMismatch {
                proof_value: proof_output.new_value,
                requested_value: new_value,
            })?;
        }

        self.value.set(&new_value, state)?;
        let stored_proof = StoredProof::from_verified_payload(proof, proof_output);
        self.last_proof.set(&stored_proof, state)?;

        self.emit_event(state, Event::NewValue(new_value));

        Ok(())
    }

    /// Sets `value` field to the `new_value`, only admin is authorized to call this method.
    pub(crate) fn set_value_and_sleep(
        &mut self,
        new_value: u32,
        sleep_millis: u64,
        proof: ValueSetterZkProofPayload,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<()> {
        self.set_value(new_value, None, proof, context, state)?;
        std::thread::sleep(std::time::Duration::from_millis(sleep_millis));
        Ok(())
    }

    pub(crate) fn set_values(
        &mut self,
        new_value: Vec<u8>,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<()> {
        let admin = self.admin.get_or_err(state)??;

        if &admin != context.sender() {
            // Here we use a custom error type.
            Err(SetValueError::WrongSender::<S> {
                admin,
                sender: context.sender().clone(),
            })?;
        }

        // This is how we set a new value:
        self.many_values.set_all(new_value, state)?;
        Ok(())
    }

    pub(crate) fn assert_visible_slot_number(
        &self,
        expected_visible_slot_number: u64,
        _context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<()> {
        let visible_height = state.current_visible_slot_number();
        anyhow::ensure!(
            visible_height.get() == expected_visible_slot_number,
            "Visible height is not as expected. Expected {}, but got {}",
            expected_visible_slot_number,
            visible_height.get()
        );
        Ok(())
    }
}
