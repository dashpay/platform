use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{PubkeyHash, ScriptBuf};
use dpp::dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Creates the owner identity of a masternode like version 1, and also decides it for the
    /// masternodes version 1 fails on.
    ///
    /// Version 1 reads a payout address and an owner address and fails with
    /// `DashCoreBadResponseError` when either is absent. Dash Core lists such masternodes from
    /// v24 on: a shared masternode has no owner, payout or collateral address (its owners are
    /// its share holders), and an extended-address masternode has a payout list instead of a
    /// payout address. This version gives:
    ///
    /// - no owner address: no owner identity (`None`), as there is no owner key to hold; the
    ///   voter and operator identities of the masternode do not depend on it;
    /// - an owner address and a single payout address (see `effective_payout_address`): the
    ///   identity of version 1, TRANSFER key id 0 and OWNER key id 1, byte for byte;
    /// - an owner address and no sole supported P2PKH payout: only the OWNER key id 1, as no
    ///   single address can hold the TRANSFER key; key id 0 stays free.
    pub(super) fn create_owner_identity_v2(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        let Some(owner_address) = masternode.state.owner_address else {
            tracing::debug!(
                pro_tx_hash = %masternode.pro_tx_hash,
                method = "create_owner_identity_v2",
                "no owner identity: the masternode has no owner address"
            );
            return Ok(None);
        };
        let owner_identifier = Self::get_owner_identifier(masternode)?;
        let mut identity = Identity::create_basic_identity(owner_identifier, platform_version)?;
        match effective_payout_address(&masternode.state) {
            Some(payout_address) => identity.add_public_keys([
                Self::get_owner_identity_withdrawal_key(payout_address, 0, platform_version)?,
                Self::get_owner_identity_owner_key(owner_address, 1, platform_version)?,
            ]),
            None => {
                tracing::debug!(
                    pro_tx_hash = %masternode.pro_tx_hash,
                    method = "create_owner_identity_v2",
                    "owner identity with only the OWNER key: no single payout address"
                );
                identity.add_public_keys([Self::get_owner_identity_owner_key(
                    owner_address,
                    1,
                    platform_version,
                )?]);
            }
        }
        Ok(Some(identity))
    }
}

/// The one address the owner reward of a masternode is paid to, which its owner identity's
/// TRANSFER key holds: the payout address, or the only entry of a payout list with one entry
/// with a matching P2PKH script (Core turns a payout address into such a list when a
/// masternode moves to extended addresses). Other scripts and reward splits have no single
/// supported TRANSFER authority.
///
/// The stored masternode list does not keep payout lists, so this is only meaningful for a
/// masternode as Core reported it, never for one read from the stored list.
fn effective_payout_address(state: &DMNState) -> Option<[u8; 20]> {
    match (state.payout_address, state.payouts.as_deref()) {
        (Some(payout_address), _) => Some(payout_address),
        (None, Some([payout]))
            if payout.script
                == ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(payout.address)) =>
        {
            Some(payout.address)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::platform::Platform;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, PubkeyHash, ScriptBuf, ScriptHash, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNPayout, DMNState, MasternodeListItem, MasternodeType,
    };
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, KeyID, Purpose};
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
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

    fn legacy_masternode() -> MasternodeListItem {
        masternode(Some(OWNER_ADDRESS), Some(PAYOUT_ADDRESS), None)
    }

    #[test]
    fn should_not_grant_transfer_authority_to_unsupported_or_inconsistent_payout_scripts() {
        for script in [
            ScriptBuf::new_p2sh(&ScriptHash::from_byte_array(PAYOUT_ADDRESS)),
            ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(SECOND_PAYOUT_ADDRESS)),
        ] {
            let identity = create_owner_identity(
                &masternode(
                    Some(OWNER_ADDRESS),
                    None,
                    Some(vec![DMNPayout {
                        address: PAYOUT_ADDRESS,
                        script,
                        reward: 10000,
                    }]),
                ),
                PlatformVersion::latest(),
            )
            .expect("owner identity");
            assert_eq!(identity.public_keys().len(), 1);
            assert_eq!(
                identity.public_keys().get(&1).expect("owner key").purpose(),
                Purpose::OWNER
            );
        }
    }

    fn create_owner_identity(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Option<Identity> {
        Platform::<MockCoreRPCLike>::create_owner_identity(masternode, platform_version)
            .expect("expected to decide the owner identity")
    }

    fn serialized(identity: Option<Identity>) -> Vec<u8> {
        identity
            .expect("expected an owner identity")
            .serialize_to_bytes()
            .expect("expected to serialize the owner identity")
    }

    /// The owner identity holds the owner's keys, and a shared masternode has no owner key
    /// (its owners are its share holders), so it gets no owner identity.
    #[test]
    fn should_create_no_owner_identity_for_a_shared_masternode() {
        let shared = masternode(None, None, None);

        assert_eq!(
            create_owner_identity(&shared, PlatformVersion::latest()),
            None
        );
    }

    /// Core turns a payout address into a payout list with one entry when a masternode moves
    /// to extended addresses, so a single payout must give the owner identity a payout address
    /// gives, which is the identity version 1 creates, byte for byte.
    #[test]
    fn should_create_the_same_owner_identity_for_a_single_payout_as_for_a_payout_address() {
        let platform_version = PlatformVersion::latest();
        let single_payout = masternode(
            Some(OWNER_ADDRESS),
            None,
            Some(vec![p2pkh_payout(PAYOUT_ADDRESS, 10000)]),
        );

        let legacy_owner_identity = serialized(create_owner_identity(
            &legacy_masternode(),
            platform_version,
        ));

        assert_eq!(
            serialized(create_owner_identity(&single_payout, platform_version)),
            legacy_owner_identity
        );
        assert_eq!(
            serialized(create_owner_identity(
                &legacy_masternode(),
                PlatformVersion::get(13).expect("expected protocol version 13"),
            )),
            legacy_owner_identity,
            "a payout address must give the identity version 1 gives"
        );
    }

    /// Several payouts have no single address for the TRANSFER key id 0, so the owner identity
    /// gets only the OWNER key id 1 a payout address's owner identity has, and key id 0 stays
    /// free.
    #[test]
    fn should_create_an_owner_identity_with_only_the_owner_key_for_several_payouts() {
        let platform_version = PlatformVersion::latest();
        let several_payouts = masternode(
            Some(OWNER_ADDRESS),
            None,
            Some(vec![
                p2pkh_payout(PAYOUT_ADDRESS, 7000),
                p2pkh_payout(SECOND_PAYOUT_ADDRESS, 3000),
            ]),
        );

        let owner_key = create_owner_identity(&legacy_masternode(), platform_version)
            .expect("expected an owner identity for a payout address")
            .public_keys()
            .get(&1)
            .expect("expected the OWNER key id 1")
            .clone();
        assert_eq!(owner_key.purpose(), Purpose::OWNER);
        assert_eq!(owner_key.data().as_slice(), OWNER_ADDRESS.as_slice());

        let several_payouts_owner_identity =
            create_owner_identity(&several_payouts, platform_version)
                .expect("expected an owner identity for several payouts");
        assert_eq!(
            several_payouts_owner_identity.id(),
            Identifier::from([0x21; 32]),
            "the owner identity id is the ProTx hash"
        );
        assert_eq!(
            several_payouts_owner_identity.public_keys(),
            &BTreeMap::from([(1 as KeyID, owner_key)])
        );
    }
}
