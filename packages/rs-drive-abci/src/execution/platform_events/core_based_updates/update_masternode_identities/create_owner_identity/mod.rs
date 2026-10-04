mod v0;
mod v1;
mod v2;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
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
    /// Versions 0 and 1 always create an identity and fail on a masternode without the payout
    /// address (and, in version 1, the owner address) they read. Version 2 (from protocol
    /// version 14) creates none for a masternode without an owner address, such as a shared
    /// masternode, and only the OWNER key for one paid to several payouts.
    ///
    /// # Arguments
    ///
    /// * masternode - A reference to the masternode list item.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<Option<Identity>, Error> - The owner identity, or `None` when the masternode
    ///   has none (only version 2 returns `None`). Otherwise, returns an error.
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
            0 => Self::create_owner_identity_v0(masternode, platform_version).map(Some),
            1 => Self::create_owner_identity_v1(masternode, platform_version).map(Some),
            2 => Self::create_owner_identity_v2(masternode, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "create_owner_identity".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::execution::ExecutionError;
    use crate::error::Error;
    use crate::platform_types::platform::Platform;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, PubkeyHash, ScriptBuf, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNPayout, DMNState, MasternodeListItem, MasternodeType,
    };
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{KeyID, Purpose};
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    const OWNER_ADDRESS: [u8; 20] = [0x22; 20];
    const PAYOUT_ADDRESS: [u8; 20] = [0x24; 20];
    const SECOND_PAYOUT_ADDRESS: [u8; 20] = [0x25; 20];

    /// A Regular masternode as Core lists it: a shared one has neither owner, payout nor
    /// collateral address; an extended-address one has `payouts` instead of `payout_address`.
    fn masternode(
        owner_address: Option<[u8; 20]>,
        payout_address: Option<[u8; 20]>,
        payouts: Option<Vec<DMNPayout>>,
    ) -> MasternodeListItem {
        MasternodeListItem {
            node_type: MasternodeType::Regular,
            pro_tx_hash: ProTxHash::from_byte_array([0x21; 32]),
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
                voting_address: [0x23; 20],
                payout_address,
                payouts,
                pub_key_operator: vec![0x26; 48],
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

    /// Generation 0 (protocol versions 1 to 3) builds the owner identity from the payout
    /// address (TRANSFER key 0), and generation 1 (4 to 13) from the payout and the owner
    /// address (TRANSFER key 0, OWNER key 1). A masternode Core lists without one of them
    /// fails with `DashCoreBadResponseError`, so its block fails instead of the masternode
    /// getting an owner identity these generations do not define: a shared masternode has
    /// neither address, and an extended-address masternode has a payout list instead of a
    /// payout address.
    #[test]
    fn should_fail_for_a_masternode_without_a_payout_or_owner_address_up_to_protocol_version_13() {
        let legacy = masternode(Some(OWNER_ADDRESS), Some(PAYOUT_ADDRESS), None);
        let shared = masternode(None, None, None);
        let single_payout = masternode(
            Some(OWNER_ADDRESS),
            None,
            Some(vec![p2pkh_payout(PAYOUT_ADDRESS, 10000)]),
        );
        let several_payouts = masternode(
            Some(OWNER_ADDRESS),
            None,
            Some(vec![
                p2pkh_payout(PAYOUT_ADDRESS, 7000),
                p2pkh_payout(SECOND_PAYOUT_ADDRESS, 3000),
            ]),
        );
        let without_owner_address = masternode(None, Some(PAYOUT_ADDRESS), None);

        for protocol_version in [3, 13] {
            let platform_version =
                PlatformVersion::get(protocol_version).expect("expected a known protocol version");

            let legacy_owner_identity_keys: BTreeMap<KeyID, (Purpose, Vec<u8>)> =
                Platform::<MockCoreRPCLike>::create_owner_identity(&legacy, platform_version)
                    .expect("expected to create the owner identity")
                    .expect("a masternode with an owner and a payout address has an owner identity")
                    .public_keys()
                    .iter()
                    .map(|(key_id, key)| (*key_id, (key.purpose(), key.data().to_vec())))
                    .collect();
            let mut expected_keys =
                BTreeMap::from([(0, (Purpose::TRANSFER, PAYOUT_ADDRESS.to_vec()))]);
            if protocol_version == 13 {
                expected_keys.insert(1, (Purpose::OWNER, OWNER_ADDRESS.to_vec()));
            }
            assert_eq!(
                legacy_owner_identity_keys, expected_keys,
                "protocol version {protocol_version}"
            );

            let mut cases = vec![
                (&shared, "has no payout address"),
                (&single_payout, "has no payout address"),
                (&several_payouts, "has no payout address"),
            ];
            if protocol_version == 13 {
                cases.push((&without_owner_address, "has no owner address"));
            }
            for (masternode, expected_message) in cases {
                match Platform::<MockCoreRPCLike>::create_owner_identity(
                    masternode,
                    platform_version,
                ) {
                    Err(Error::Execution(ExecutionError::DashCoreBadResponseError(message))) => {
                        assert!(
                            message.ends_with(expected_message),
                            "protocol version {protocol_version}: unexpected message {message}"
                        )
                    }
                    other => panic!(
                        "protocol version {protocol_version}: expected DashCoreBadResponseError \
                         for {masternode:?}, got {other:?}"
                    ),
                }
            }
        }
    }
}
