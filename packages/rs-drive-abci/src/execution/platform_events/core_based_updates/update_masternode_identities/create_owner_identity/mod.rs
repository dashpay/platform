mod v0;
mod v1;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem};
use dpp::identity::Identity;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Creates an owner identity based on the given masternode list item.
    ///
    /// This function constructs an identity for an owner using details from the masternode.
    /// It delegates to a version-specific method depending on the platform version.
    ///
    /// # Arguments
    ///
    /// * masternode - A reference to the masternode list item.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<Option<Identity>, Error> - The owner identity, or `None` when the masternode
    ///   has no key an owner identity of this version holds: a shared masternode has no owner
    ///   and no payout address. Otherwise, returns an error.
    pub(crate) fn create_owner_identity(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .create_owner_identity
        {
            0 => Self::create_owner_identity_v0(masternode, platform_version),
            1 => Self::create_owner_identity_v1(masternode, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "create_owner_identity".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

/// The one address the owner reward of a masternode is paid to, which its owner identity's
/// TRANSFER key holds: the payout address, or the only entry of a payout list with one entry
/// (Core turns a payout address into such a list when a masternode moves to extended
/// addresses). `None` for a shared masternode, which has neither, and for a reward split
/// between several payouts.
///
/// The stored masternode list does not keep payout lists, so this is only meaningful for a
/// masternode as Core reported it, never for one read from the stored list.
///
/// Shared by the shipped `create_owner_identity` v0 and v1: changing it changes both.
fn effective_payout_address(state: &DMNState) -> Option<[u8; 20]> {
    match (state.payout_address, state.payouts.as_deref()) {
        (Some(payout_address), _) => Some(payout_address),
        (None, Some([payout])) => Some(payout.address),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, PubkeyHash, ScriptBuf, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNPayout, DMNState, MasternodeListDiff, MasternodeListItem, MasternodeType,
    };
    use dpp::identifier::MasternodeIdentifiers;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, KeyID, Purpose};
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    const PRO_TX_HASH: [u8; 32] = [0x21; 32];
    const OWNER_ADDRESS: [u8; 20] = [0x22; 20];
    const VOTING_ADDRESS: [u8; 20] = [0x23; 20];
    const PAYOUT_ADDRESS: [u8; 20] = [0x24; 20];
    const SECOND_PAYOUT_ADDRESS: [u8; 20] = [0x25; 20];
    const PUB_KEY_OPERATOR: [u8; 48] = [0x26; 48];

    /// A Regular masternode as Core lists it: a shared one has neither owner, payout nor
    /// collateral address; an extended-address one has `payouts` instead of `payout_address`.
    fn masternode(
        owner_address: Option<[u8; 20]>,
        payout_address: Option<[u8; 20]>,
        payouts: Option<Vec<DMNPayout>>,
    ) -> MasternodeListItem {
        MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash: ProTxHash::from_byte_array(PRO_TX_HASH),
            collateral_hash: Txid::from_byte_array([0x27; 32]),
            collateral_index: 0,
            collateral_address: owner_address.map(|_| [0x28; 20]),
            operator_reward: 0.0,
            state: DMNState {
                service: "1.2.3.4:9999".parse().expect("socket address"),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address,
                voting_address: VOTING_ADDRESS,
                payout_address,
                payouts,
                pub_key_operator: PUB_KEY_OPERATOR.to_vec(),
                operator_payout_address: None,
                platform_node_id: None,
                #[allow(deprecated)]
                legacy_platform_p2p_port: None,
                #[allow(deprecated)]
                legacy_platform_http_port: None,
                addresses: None,
            },
        }
    }

    fn p2pkh_payout(key_hash: [u8; 20], reward: u16) -> DMNPayout {
        DMNPayout {
            address: key_hash,
            script: ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(key_hash)),
            reward,
        }
    }

    fn shared_masternode() -> MasternodeListItem {
        masternode(None, None, None)
    }

    fn legacy_masternode() -> MasternodeListItem {
        masternode(Some(OWNER_ADDRESS), Some(PAYOUT_ADDRESS), None)
    }

    fn single_payout_masternode() -> MasternodeListItem {
        masternode(
            Some(OWNER_ADDRESS),
            None,
            Some(vec![p2pkh_payout(PAYOUT_ADDRESS, 10000)]),
        )
    }

    fn multi_payout_masternode() -> MasternodeListItem {
        masternode(
            Some(OWNER_ADDRESS),
            None,
            Some(vec![
                p2pkh_payout(PAYOUT_ADDRESS, 7000),
                p2pkh_payout(SECOND_PAYOUT_ADDRESS, 3000),
            ]),
        )
    }

    /// A platform on `platform_version` whose masternode identities were created for
    /// `masternode`, added on the first block.
    fn platform_with_added_masternode(
        masternode: MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> TempPlatform<MockCoreRPCLike> {
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        platform
            .update_masternode_identities(
                MasternodeListDiff {
                    base_height: 0,
                    block_height: 1,
                    added_mns: vec![masternode],
                    removed_mns: vec![],
                    updated_mns: vec![],
                },
                &BTreeMap::new(),
                &BlockInfo::default(),
                None,
                &transaction,
                platform_version,
            )
            .expect("expected to create the masternode identities");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
        platform
    }

    fn fetch_identity(
        platform: &TempPlatform<MockCoreRPCLike>,
        identity_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Option<Identity> {
        platform
            .drive
            .fetch_full_identity(identity_id.to_buffer(), None, platform_version)
            .expect("expected to fetch an identity")
    }

    fn owner_identity(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> Option<Identity> {
        fetch_identity(platform, PRO_TX_HASH.into(), platform_version)
    }

    fn assert_voter_and_operator_identities_exist(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) {
        let voter_identifier = Identifier::create_voter_identifier(&PRO_TX_HASH, &VOTING_ADDRESS);
        let operator_identifier =
            Identifier::create_operator_identifier(&PRO_TX_HASH, &PUB_KEY_OPERATOR);
        assert!(
            fetch_identity(platform, voter_identifier, platform_version).is_some(),
            "the voter identity must be created"
        );
        assert!(
            fetch_identity(platform, operator_identifier, platform_version).is_some(),
            "the operator identity must be created"
        );
    }

    /// Both generations: the owner identity carries the owner's keys, and a shared
    /// masternode has no owner key (its owners are its share holders), so it gets no owner
    /// identity. Its voter and operator identities are created as for any masternode.
    #[test]
    fn should_create_no_owner_identity_for_a_shared_masternode() {
        for platform_version in [PlatformVersion::first(), PlatformVersion::latest()] {
            let platform = platform_with_added_masternode(shared_masternode(), platform_version);

            assert_eq!(
                owner_identity(&platform, platform_version),
                None,
                "protocol version {}",
                platform_version.protocol_version
            );
            assert_voter_and_operator_identities_exist(&platform, platform_version);
        }
    }

    /// Both generations: Core turns a payout address into a payout list with one entry
    /// when a masternode moves to extended addresses, so a single payout must give the
    /// owner identity a payout address gives, byte for byte.
    #[test]
    fn should_create_the_same_owner_identity_for_a_single_payout_as_for_a_payout_address() {
        for platform_version in [PlatformVersion::first(), PlatformVersion::latest()] {
            let legacy_owner_identity = owner_identity(
                &platform_with_added_masternode(legacy_masternode(), platform_version),
                platform_version,
            )
            .expect("expected an owner identity for a payout address");
            let single_payout_owner_identity = owner_identity(
                &platform_with_added_masternode(single_payout_masternode(), platform_version),
                platform_version,
            )
            .expect("expected an owner identity for a single payout");

            assert_eq!(
                single_payout_owner_identity
                    .serialize_to_bytes()
                    .expect("expected to serialize"),
                legacy_owner_identity
                    .serialize_to_bytes()
                    .expect("expected to serialize"),
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }

    /// Several payouts have no single address for the TRANSFER key id 0, so the owner
    /// identity gets only the OWNER key id 1 a payout address's owner identity has, and key
    /// id 0 stays free.
    #[test]
    fn should_create_an_owner_identity_with_only_the_owner_key_for_several_payouts() {
        let platform_version = PlatformVersion::latest();
        let multi_payout_owner_identity = owner_identity(
            &platform_with_added_masternode(multi_payout_masternode(), platform_version),
            platform_version,
        )
        .expect("expected an owner identity for several payouts");
        let legacy_owner_identity = owner_identity(
            &platform_with_added_masternode(legacy_masternode(), platform_version),
            platform_version,
        )
        .expect("expected an owner identity for a payout address");

        let owner_key = legacy_owner_identity
            .public_keys()
            .get(&1)
            .expect("expected the OWNER key id 1")
            .clone();
        assert_eq!(owner_key.purpose(), Purpose::OWNER);
        assert_eq!(owner_key.data().as_slice(), OWNER_ADDRESS.as_slice());
        assert_eq!(
            multi_payout_owner_identity.public_keys(),
            &BTreeMap::from([(1 as KeyID, owner_key)])
        );
    }

    /// The first generation's owner identity holds only the TRANSFER key of the payout
    /// address, so several payouts leave it with no key: no owner identity is created, as
    /// for a shared masternode.
    #[test]
    fn should_create_no_owner_identity_for_several_payouts_in_protocol_versions_without_owner_keys()
    {
        let platform_version = PlatformVersion::first();
        let platform = platform_with_added_masternode(multi_payout_masternode(), platform_version);

        assert_eq!(owner_identity(&platform, platform_version), None);
        assert_voter_and_operator_identities_exist(&platform, platform_version);
    }
}
