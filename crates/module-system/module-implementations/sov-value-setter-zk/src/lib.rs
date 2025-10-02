#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
mod call;
mod event;
mod genesis;
mod hooks;
mod proof;

pub use call::*;
pub use event::Event;
pub use genesis::*;
pub use proof::{StoredProof, ValueSetterZkProofPayload, ValueSetterZkProofPublicOutput};
use sov_modules_api::{
    AccessoryStateValue, Context, DaSpec, Gas, GenesisState, Module, ModuleId, ModuleInfo,
    ModuleRestApi, Spec, StateValue, StateVec, TxState,
};

/// A value setter module which requires a valid RISC0 proof to update the stored value.
#[derive(Clone, ModuleInfo, ModuleRestApi)]
pub struct ValueSetterZk<S: Spec> {
    /// The ID of the module.
    #[id]
    pub id: ModuleId,

    /// The value kept in the state.
    #[state]
    pub value: StateValue<u32>,

    /// The last successfully verified proof corresponding to `value`.
    #[state]
    pub last_proof: StateValue<StoredProof>,

    /// Additional values kept in state for compatibility with the original value setter module.
    #[state]
    pub many_values: StateVec<u8>,

    /// The number of times the `begin_slot` hook has been called.
    #[state]
    pub begin_rollup_block_hook_count: StateValue<u32>,

    /// The number of times the `end_slot` hook has been called.
    #[state]
    pub end_rollup_block_hook_count: StateValue<u32>,

    /// The number of times the `finalize` hook has been called.
    #[state]
    pub finalize_hook_count: AccessoryStateValue<u32>,

    /// Holds the address of the admin user who is allowed to update the value.
    #[state]
    pub admin: StateValue<S::Address>,
}

/// Gas configuration for the value setter zk module
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct ValueSetterZkGasConfig<GU: Gas> {
    /// Gas price multiplier for the set_value operation
    pub set_value: GU,
}

impl<S: Spec> Module for ValueSetterZk<S> {
    type Spec = S;

    type Config = ValueSetterZkConfig<S>;

    type CallMessage = CallMessage<S>;

    type Event = Event;

    fn genesis(
        &mut self,
        _genesis_rollup_header: &<<S as Spec>::Da as DaSpec>::BlockHeader,
        config: &Self::Config,
        state: &mut impl GenesisState<S>,
    ) -> anyhow::Result<()> {
        self.init_module(config, state)
    }

    fn call(
        &mut self,
        msg: Self::CallMessage,
        context: &Context<Self::Spec>,
        state: &mut impl TxState<S>,
    ) -> anyhow::Result<()> {
        let mut state_wrapped = state.to_revertable();
        let state = &mut state_wrapped;
        let res = match msg {
            CallMessage::SetValue {
                value: new_value,
                gas,
                proof,
            } => Ok(self.set_value(new_value, gas, proof, context, state)?),
            CallMessage::SetValueAndSleep {
                value: new_value,
                sleep_millis,
                proof,
            } => Ok(self.set_value_and_sleep(new_value, sleep_millis, proof, context, state)?),
            CallMessage::SetManyValues(many) => Ok(self.set_values(many, context, state)?),
            CallMessage::AssertVisibleSlotNumber {
                expected_visible_slot_number,
            } => {
                Ok(self.assert_visible_slot_number(expected_visible_slot_number, context, state)?)
            }
            CallMessage::Panic => {
                panic!("sov_value_setter_zk: Panic requested by user sending a panic message");
            }
        };
        state_wrapped.commit();
        res
    }
}
