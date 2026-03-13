use crate::drive::Drive;
use grovedb::Element::Item;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::serialization::PlatformDeserializable;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_token_direct_selling_prices_v0<
        T: FromIterator<(I, Option<TokenPricingSchedule>)>,
        I: From<[u8; 32]>,
    >(
        proof: &[u8],
        token_ids: &[[u8; 32]],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::token_direct_purchase_prices_query(token_ids);
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };
        if proved_key_values.len() == token_ids.len() {
            let values = proved_key_values
                .into_iter()
                .map(|proved_key_value| {
                    let token_id: [u8; 32] = proved_key_value.1.try_into().map_err(|_| {
                        Error::Proof(ProofError::IncorrectValueSize("token id size"))
                    })?;
                    match proved_key_value.2 {
                        Some(Item(value, ..)) => Ok((
                            token_id.into(),
                            Some(TokenPricingSchedule::deserialize_from_bytes(&value)?),
                        )),
                        None => Ok((token_id.into(), None)),
                        _ => Err(Error::Proof(ProofError::IncorrectValueSize(
                            "proof did not point to an item as expected for token info",
                        ))),
                    }
                })
                .collect::<Result<T, Error>>()?;
            Ok((root_hash, values))
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: token_ids.len(),
                got: proved_key_values.len(),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::prelude::DataContract;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_and_verify_multiple_token_direct_selling_prices() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract = DataContract::V1(DataContractV1 {
            id: Default::default(),
            version: 0,
            owner_id: Default::default(),
            document_types: Default::default(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: Default::default(),
            tokens: BTreeMap::from([
                (
                    0,
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
                ),
                (
                    1,
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
                ),
            ]),
            keywords: Vec::new(),
            description: None,
        });

        let token_id_1 = contract.token_id(0).expect("expected token at position 0");
        let token_id_2 = contract.token_id(1).expect("expected token at position 1");

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Set price for token 1 only
        let price_1 = TokenPricingSchedule::SinglePrice(2000);
        drive
            .token_set_direct_purchase_price(
                token_id_1.to_buffer(),
                Some(price_1.clone()),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to set direct purchase price");

        // Generate proof using the multi-token prove function
        let proof = drive
            .prove_tokens_direct_purchase_price(
                &[token_id_1.to_buffer(), token_id_2.to_buffer()],
                None,
                platform_version,
            )
            .expect("expected to get proof");

        let (_, verified_prices): (_, BTreeMap<[u8; 32], Option<TokenPricingSchedule>>) =
            Drive::verify_token_direct_selling_prices(
                proof.as_slice(),
                &[token_id_1.to_buffer(), token_id_2.to_buffer()],
                false,
                platform_version,
            )
            .expect("expected proof verification to succeed");

        assert_eq!(
            verified_prices,
            BTreeMap::from([
                (token_id_1.to_buffer(), Some(price_1)),
                (token_id_2.to_buffer(), None),
            ])
        );
    }
}
