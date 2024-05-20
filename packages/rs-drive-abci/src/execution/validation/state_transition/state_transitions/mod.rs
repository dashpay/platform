/// Module containing functionality related to batch processing of documents.
pub mod documents_batch;

/// Module for creating an identity entity.
pub mod identity_create;

/// Module for managing transfers of credit between identity entities.
pub mod identity_credit_transfer;

/// Module for managing withdrawals of credit from an identity entity.
pub mod identity_credit_withdrawal;

/// Module for topping up credit in an identity entity.
pub mod identity_top_up;

/// Module for updating an existing identity entity.
pub mod identity_update;

/// Module for creating a data contract entity.
pub mod data_contract_create;

/// Module for updating an existing data contract entity.
pub mod data_contract_update;

/// Module for voting from a masternode.
pub mod masternode_vote;

/// The validation mode we are using
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationMode {
    /// The basic checktx before the state transition is put into mempool
    CheckTx,
    /// Rechecking a state transition every block
    RecheckTx,
    /// The validation of the validator
    Validator,
    /// A validation mode used to get the action with no validation
    NoValidation,
}

impl ValidationMode {
    /// Can this validation mode alter cache on drive?
    pub fn can_alter_cache(&self) -> bool {
        match self {
            ValidationMode::CheckTx => false,
            ValidationMode::RecheckTx => false,
            ValidationMode::Validator => true,
            ValidationMode::NoValidation => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::block::block_info::BlockInfo;
    use dpp::fee::Credits;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
    use dpp::prelude::Identifier;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    pub(in crate::execution::validation::state_transition::state_transitions) fn setup_identity(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seed: u64,
        credits: Credits,
    ) -> (Identity, SimpleSigner, IdentityPublicKey) {
        let platform_version = PlatformVersion::latest();
        let mut signer = SimpleSigner::default();

        let mut rng = StdRng::seed_from_u64(seed);

        let (master_key, master_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                0,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_key(master_key.clone(), master_private_key.clone());

        let (critical_public_key, private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                1,
                &mut rng,
                platform_version,
            )
            .expect("expected to get key pair");

        signer.add_key(critical_public_key.clone(), private_key.clone());

        let identity: Identity = IdentityV0 {
            id: Identifier::random_with_rng(&mut rng),
            public_keys: BTreeMap::from([
                (0, master_key.clone()),
                (1, critical_public_key.clone()),
            ]),
            balance: credits,
            revision: 0,
        }
        .into();

        // We just add this identity to the system first

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add a new identity");

        (identity, signer, critical_public_key)
    }
}
