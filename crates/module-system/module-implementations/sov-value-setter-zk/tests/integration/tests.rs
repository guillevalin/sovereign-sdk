use sov_modules_api::macros::UniversalWallet;
use sov_modules_api::sov_universal_wallet::schema::Schema;
use sov_modules_api::Error::ModuleError;
use sov_test_utils::runtime::genesis::zk::config::HighLevelZkGenesisConfig;
use sov_test_utils::runtime::{TestRunner, ValueSetterZk};
use sov_test_utils::{generate_zk_runtime, AsUser, TestSpec, TestUser, TransactionTestCase};
use sov_value_setter_zk::proof::{install_mock_proof_verifier, ProofVerificationError};
use sov_value_setter_zk::{
    CallMessage, Event, SetValueError, ValueSetterZkConfig, ValueSetterZkProofPayload,
    ValueSetterZkProofPublicOutput,
};

generate_zk_runtime!(TestRuntime <= value_setter: ValueSetterZk<S>);

type S = TestSpec;
type RT = TestRuntime<S>;

fn setup() -> (TestRunner<TestRuntime<S>, S>, TestUser<S>, TestUser<S>) {
    let genesis_config = HighLevelZkGenesisConfig::generate_with_additional_accounts(2);

    let admin_account = genesis_config.additional_accounts()[0].clone();
    let extra_account = genesis_config.additional_accounts()[1].clone();

    let genesis = GenesisConfig::from_minimal_config(
        genesis_config.clone().into(),
        ValueSetterZkConfig {
            admin: admin_account.address(),
        },
    );

    (
        TestRunner::new_with_genesis(genesis.into_genesis_params(), Default::default()),
        admin_account,
        extra_account,
    )
}

#[test]
fn test_setting_value() {
    let (mut runner, admin, _) = setup();
    let _guard = install_mock_proof_verifier(|payload| {
        decode_proof(payload).map(|value| ValueSetterZkProofPublicOutput { new_value: value })
    });

    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, ValueSetterZk<S>>(CallMessage::SetValue {
            value: 5,
            gas: None,
            proof: dummy_proof(5),
        }),
        assert: Box::new(|result, state| {
            assert_eq!(
                ValueSetterZk::<S>::default().value.get(state).unwrap(),
                Some(5)
            );
            assert!(result.events.iter().any(|event| matches!(
                event,
                TestRuntimeEvent::ValueSetter(Event::NewValue(value)) if *value == 5
            )));
        }),
    });
}

#[test]
fn test_setting_value_not_admin() {
    let (mut runner, admin, non_admin) = setup();
    let _guard = install_mock_proof_verifier(|payload| {
        decode_proof(payload).map(|value| ValueSetterZkProofPublicOutput { new_value: value })
    });

    runner.execute_transaction(TransactionTestCase {
        input: non_admin.create_plain_message::<RT, ValueSetterZk<S>>(CallMessage::SetValue {
            value: 5,
            gas: None,
            proof: dummy_proof(5),
        }),
        assert: Box::new(move |result, _state| {
            match &result.tx_receipt {
                sov_modules_api::TxEffect::Reverted(reason) => {
                    assert_eq!(
                        &reason.reason,
                        &ModuleError(
                            SetValueError::<S>::WrongSender {
                                sender: non_admin.address(),
                                admin: admin.address(),
                            }
                            .into()
                        ),
                        "Transaction reverted, but with unexpected reason"
                    );
                }
                unexpected => panic!("Expected transaction to revert, but got: {unexpected:?}"),
            };
        }),
    });
}

#[test]
fn test_display_value_setter_call() {
    #[derive(Debug, Clone, PartialEq, borsh::BorshSerialize, UniversalWallet)]
    enum RuntimeCall {
        ValueSetter(CallMessage<S>),
    }

    let msg = RuntimeCall::ValueSetter(CallMessage::SetValue {
        value: 92,
        gas: None,
        proof: dummy_proof(92),
    });

    let schema = Schema::of_single_type::<RuntimeCall>().unwrap();
    let rendered = schema.display(0, &borsh::to_vec(&msg).unwrap()).unwrap();
    assert!(rendered.contains("ValueSetter.SetValue"));
    assert!(rendered.contains("value: 92"));
}

#[test]
fn test_proof_value_mismatch_reverts() {
    let (mut runner, admin, _) = setup();
    let _guard = install_mock_proof_verifier(|payload| {
        decode_proof(payload).map(|value| ValueSetterZkProofPublicOutput { new_value: value })
    });

    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, ValueSetterZk<S>>(CallMessage::SetValue {
            value: 7,
            gas: None,
            proof: dummy_proof(5),
        }),
        assert: Box::new(|result, _state| match &result.tx_receipt {
            sov_modules_api::TxEffect::Reverted(reason) => {
                assert_eq!(
                    &reason.reason,
                    &ModuleError(
                        SetValueError::<S>::ProofValueMismatch {
                            proof_value: 5,
                            requested_value: 7,
                        }
                        .into()
                    ),
                );
            }
            unexpected => panic!("Expected mismatch to revert, got: {unexpected:?}"),
        }),
    });
}

fn dummy_proof(value: u32) -> ValueSetterZkProofPayload {
    ValueSetterZkProofPayload {
        method_id: [0u8; 32],
        proof: value.to_le_bytes().to_vec(),
    }
}

fn decode_proof(payload: &ValueSetterZkProofPayload) -> Result<u32, ProofVerificationError> {
    if payload.proof.len() < 4 {
        return Err(ProofVerificationError::VerificationFailed {
            reason: "proof too short".to_owned(),
        });
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&payload.proof[..4]);
    Ok(u32::from_le_bytes(bytes))
}
