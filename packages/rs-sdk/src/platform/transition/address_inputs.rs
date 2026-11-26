use crate::platform::FetchMany;
use crate::{Error, Sdk};
use dpp::address_funds::PlatformAddress;
use dpp::errors::consensus::basic::state_transition::TransitionNoInputsError;
use dpp::errors::consensus::state::address_funds::address_does_not_exist_error::AddressDoesNotExistError;
use dpp::errors::consensus::state::address_funds::address_not_enough_funds_error::AddressNotEnoughFundsError;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use drive_proof_verifier::types::{AddressInfo, AddressInfos};
use std::collections::{BTreeMap, BTreeSet};

/// Fetch each input address's current `(nonce, balance)` from Platform
/// and return `(nonce, amount)` per address, enforcing a hard balance
/// check — errors with `AddressNotEnoughFundsError` when any input is
/// short rather than letting an underfunded transition proceed. The
/// returned nonces are the *current* on-chain values; callers increment
/// them (see [`nonce_inc`], or apply their own checked increment) before
/// building a transition.
pub async fn fetch_inputs_with_nonce(
    sdk: &Sdk,
    amounts: &BTreeMap<PlatformAddress, Credits>,
) -> Result<BTreeMap<PlatformAddress, (AddressNonce, Credits)>, Error> {
    if amounts.is_empty() {
        return Err(Error::from(TransitionNoInputsError::new()));
    }

    let addresses: BTreeSet<PlatformAddress> = amounts.keys().copied().collect();
    let address_infos = AddressInfo::fetch_many(sdk, addresses).await?;

    let mut inputs_with_nonce = BTreeMap::new();
    for (address, amount) in amounts {
        let info = ensure_address_exists(&address_infos, *address)?;
        ensure_address_balance(*address, info.balance, *amount)?;
        inputs_with_nonce.insert(*address, (info.nonce, *amount));
    }

    Ok(inputs_with_nonce)
}

/// Increments the nonce for each address in the provided map.
pub(crate) fn nonce_inc(
    data: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
) -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
    data.into_iter()
        .map(|(address, (nonce, credits))| (address, (nonce + 1, credits)))
        .collect()
}

/// Validates that the provided `address_infos_map` contains exactly the set of `expected_addresses`
/// and converts it into [`AddressInfos`].
pub(crate) fn collect_address_infos_from_proof(
    address_infos_map: BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
    expected_addresses: &BTreeSet<PlatformAddress>,
) -> Result<AddressInfos, Error> {
    let returned_addresses: BTreeSet<PlatformAddress> = address_infos_map.keys().copied().collect();

    if expected_addresses.len() != returned_addresses.len() {
        tracing::debug!(
            ?expected_addresses,
            ?returned_addresses,
            "address proof length mismatch",
        );
        return Err(Error::InvalidProvedResponse(format!(
            "proof returned different number of addresses. expected {}, received {}",
            expected_addresses.len(),
            address_infos_map.len()
        )));
    }

    let address_infos_keys: BTreeSet<&PlatformAddress> = address_infos_map.keys().collect();
    let expected_addresses_ref: BTreeSet<&PlatformAddress> =
        expected_addresses.iter().by_ref().collect();

    if address_infos_keys != expected_addresses_ref {
        tracing::debug!(
            ?expected_addresses_ref,
            ?address_infos_keys,
            "address proof mismatch",
        );
        return Err(Error::InvalidProvedResponse(
            "proof returned different addresses".to_string(),
        ));
    }
    let infos: AddressInfos = address_infos_map
        .into_iter()
        .map(|(address, maybe_info)| {
            let info = maybe_info.map(|(nonce, balance)| AddressInfo {
                address,
                nonce,
                balance,
            });
            (address, info)
        })
        .collect();

    Ok(infos)
}

fn ensure_address_exists(
    infos: &AddressInfos,
    address: PlatformAddress,
) -> Result<&AddressInfo, Error> {
    infos
        .get(&address)
        .ok_or_else(|| Error::from(AddressDoesNotExistError::new(address)))?
        .as_ref()
        .ok_or_else(|| Error::from(AddressDoesNotExistError::new(address)))
}

fn ensure_address_balance(
    address: PlatformAddress,
    available: Credits,
    required: Credits,
) -> Result<(), Error> {
    if available < required {
        Err(Error::from(AddressNotEnoughFundsError::new(
            address, available, required,
        )))
    } else {
        Ok(())
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::error::StaleNodeError;
    use crate::SdkBuilder;
    use dapi_grpc::platform::v0::ResponseMetadata;
    use dpp::version::PlatformVersion;

    #[tokio::test]
    async fn should_reject_address_input_nonce_snapshot_one_block_behind_observed_height() {
        let mut sdk = SdkBuilder::new_mock().build().unwrap();
        let address = PlatformAddress::P2pkh([0x10; 20]);
        let amounts = BTreeMap::from([(address, 100)]);
        // Mock proof expectations report height 0. Model a prior transition
        // observed one block later, after it consumed nonce 1.
        sdk.mock()
            .expect_fetch_many::<PlatformAddress, AddressInfo, _, AddressInfos>(
                BTreeSet::from([address]),
                Some(
                    [(
                        address,
                        Some(AddressInfo {
                            address,
                            nonce: 0,
                            balance: 1_000,
                        }),
                    )]
                    .into_iter()
                    .collect(),
                ),
            )
            .await
            .unwrap();
        sdk.clone()
            .verify_response_metadata(
                "wait_for_state_transition_result",
                &ResponseMetadata {
                    height: 1,
                    protocol_version: PlatformVersion::latest().protocol_version,
                    ..Default::default()
                },
            )
            .unwrap();

        let error = fetch_inputs_with_nonce(&sdk, &amounts)
            .await
            .expect_err("must not build nonce 1 again from the previous block");
        assert!(
            matches!(
                error,
                Error::StaleNode(StaleNodeError::Height {
                    expected_height: 1,
                    received_height: 0,
                    tolerance_blocks: 0,
                })
            ),
            "unexpected error: {error:?}"
        );
    }
}
