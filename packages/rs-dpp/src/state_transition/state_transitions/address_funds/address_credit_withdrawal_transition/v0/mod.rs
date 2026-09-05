mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::{AddressNonce, UserFeeIncrease};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::{identity::core_script::CoreScript, withdrawal::Pooling, ProtocolError};

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, Encode, Decode, PlatformSignable, PartialEq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct AddressCreditWithdrawalTransitionV0 {
    #[cfg_attr(
        feature = "json-conversion",
        serde(with = "crate::address_funds::serde_helpers::address_input_map")
    )]
    pub inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// Optional output for change
    #[cfg_attr(
        feature = "json-conversion",
        serde(with = "crate::address_funds::serde_helpers::address_output_singular")
    )]
    pub output: Option<(PlatformAddress, Credits)>,
    pub fee_strategy: AddressFundsFeeStrategy,
    pub core_fee_per_byte: u32,
    #[cfg_attr(
        feature = "serde-conversion",
        serde(with = "crate::withdrawal::pooling_serde")
    )]
    pub pooling: Pooling,
    pub output_script: CoreScript,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub input_witnesses: Vec<AddressWitness>,
}

#[cfg(all(test, feature = "state-transition-signing"))]
mod signing_tests {
    use super::*;
    use crate::address_funds::AddressFundsFeeStrategyStep;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::identity::signer::Signer;
    use crate::state_transition::address_credit_withdrawal_transition::methods::AddressCreditWithdrawalTransitionMethodsV0;
    use async_trait::async_trait;
    use platform_value::BinaryData;
    use platform_version::version::PlatformVersion;

    #[derive(Debug)]
    struct UnreachableAddressSigner;

    #[async_trait]
    impl Signer<PlatformAddress> for UnreachableAddressSigner {
        async fn sign(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            panic!("sign should not run when protocol gating rejects the constructor")
        }

        async fn sign_create_witness(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            panic!(
                "sign_create_witness should not run when protocol gating rejects the constructor"
            )
        }

        fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn constructor_returns_not_active_before_structure_validation() {
        let mut low_version = PlatformVersion::get(1)
            .expect("platform version 1 exists")
            .clone();
        low_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .address_credit_withdrawal
            .basic_structure = None;
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (1, 1));

        let result = AddressCreditWithdrawalTransitionV0::try_from_inputs_with_signer(
            inputs,
            None,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(99)],
            1,
            crate::withdrawal::Pooling::Never,
            CoreScript::default(),
            &UnreachableAddressSigner,
            0,
            &low_version,
        )
        .await;

        assert!(matches!(
            result,
            Err(ProtocolError::ConsensusError(boxed))
                if matches!(
                    *boxed,
                    ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(_))
                )
        ));
    }
}
