use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::validation::ConsensusValidationResult;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl IdentityCreditWithdrawalTransitionAction {
    /// try from an identity credit withdrawal
    pub fn try_from_identity_credit_withdrawal(
        drive: &Drive,
        tx: TransactionArg,
        identity_credit_withdrawal: &IdentityCreditWithdrawalTransition,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, Error> {
        match identity_credit_withdrawal {
            IdentityCreditWithdrawalTransition::V0(v0) => {
                Ok(ConsensusValidationResult::new_with_data(
                    IdentityCreditWithdrawalTransitionActionV0::from_identity_credit_withdrawal_v0(
                        v0,
                        block_info.time_ms,
                    )
                    .into(),
                ))
            }
            IdentityCreditWithdrawalTransition::V1(v1) => {
                IdentityCreditWithdrawalTransitionActionV0::try_from_identity_credit_withdrawal_v1(
                    drive,
                    tx,
                    v1,
                    block_info,
                    platform_version,
                )
                .map(|consensus_validation_result| {
                    consensus_validation_result.map(|withdrawal| withdrawal.into())
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;

    #[test]
    fn test_try_from_identity_credit_withdrawal_without_address_and_multiple_transfer_keys() {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let mut identity = Identity::random_identity(1, Some(64), platform_version)
            .expect("create random identity");

        let (transfer_key1, _) =
            IdentityPublicKey::random_masternode_transfer_key(2, Some(64), platform_version)
                .expect("create random masternode transfer key");
        let (transfer_key2, _) =
            IdentityPublicKey::random_masternode_transfer_key(3, Some(64), platform_version)
                .expect("create random masternode transfer key");

        identity.add_public_keys([transfer_key1, transfer_key2]);

        let identity_id = identity.id();

        drive
            .add_new_identity(
                identity,
                true,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("create new identity");

        let transition =
            IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
                identity_id,
                nonce: 0,
                amount: 0,
                core_fee_per_byte: 0,
                pooling: Default::default(),
                output_script: None,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            });

        IdentityCreditWithdrawalTransitionAction::try_from_identity_credit_withdrawal(
            &drive,
            None,
            &transition,
            &BlockInfo::default(),
            platform_version,
        )
        .expect("create action");
    }
}
