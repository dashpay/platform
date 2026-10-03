use super::effective_payout_address;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn create_owner_identity_v1(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        // Only a Core v24 masternode list has a masternode without an owner address or without
        // a single payout address, and no earlier binary can parse one, so no committed block
        // reached the branches below that skip a key.
        //
        // A shared masternode has no owner key: its owners are its share holders.
        let Some(owner_address) = masternode.state.owner_address else {
            tracing::debug!(
                pro_tx_hash = %masternode.pro_tx_hash,
                method = "create_owner_identity_v1",
                "no owner identity: the masternode has no owner address"
            );
            return Ok(None);
        };
        let owner_identifier = Self::get_owner_identifier(masternode)?;
        let mut identity = Identity::create_basic_identity(owner_identifier, platform_version)?;
        // Several payouts have no single address for the TRANSFER key, so the identity holds
        // only the OWNER key, under the same id as next to a TRANSFER key.
        if let Some(payout_address) = effective_payout_address(&masternode.state) {
            identity.add_public_keys([Self::get_owner_identity_withdrawal_key(
                payout_address,
                0,
                platform_version,
            )?]);
        } else {
            tracing::debug!(
                pro_tx_hash = %masternode.pro_tx_hash,
                method = "create_owner_identity_v1",
                "owner identity with only the OWNER key: no single payout address"
            );
        }
        identity.add_public_keys([Self::get_owner_identity_owner_key(
            owner_address,
            1,
            platform_version,
        )?]);
        Ok(Some(identity))
    }
}
