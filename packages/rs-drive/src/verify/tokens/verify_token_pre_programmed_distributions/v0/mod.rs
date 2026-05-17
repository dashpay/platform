use crate::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use grovedb::Element::SumItem;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Version 0: Verifies the pre-programmed token distributions using a cryptographic proof.
    ///
    /// This function checks the proof and reconstructs the token’s pre-programmed distributions
    /// using generics to allow flexibility in return types.
    ///
    /// # Parameters
    /// - `proof`: The cryptographic proof.
    /// - `token_id`: The ID of the token (32-byte array).
    /// - `verify_subset_of_proof`: Whether to verify only a subset of the proof.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// - `Ok((RootHash, T))`:
    ///   - `RootHash`: The verified root hash of the database.
    ///   - `T`: A collection implementing `FromIterator<(TimestampMillis, D)>` where `D`
    ///     implements `FromIterator<(Identifier, TokenAmount)>`.
    pub(super) fn verify_token_pre_programmed_distributions_v0<
        T: FromIterator<(TimestampMillis, D)>,
        D: FromIterator<(Identifier, TokenAmount)>,
    >(
        proof: &[u8],
        token_id: [u8; 32],
        start_at: Option<QueryPreProgrammedDistributionStartAt>,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Drive::pre_programmed_distributions_query(token_id, start_at, limit);

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        // Group values by TimestampMillis first
        let grouped_data = proved_key_values.into_iter().try_fold(
            BTreeMap::<TimestampMillis, Vec<(Identifier, TokenAmount)>>::new(),
            |mut acc, (mut path, key, element)| {
                let time_bytes = path.pop().ok_or_else(|| {
                    Error::Proof(ProofError::IncorrectElementPath {
                        expected: path_query.path.clone(),
                        actual: path.clone(),
                    })
                })?;

                if time_bytes.len() != 8 {
                    return Err(Error::Proof(ProofError::IncorrectValueSize(
                        "time key in pre-programmed distributions is not 8 bytes",
                    )));
                }

                let time = TimestampMillis::from_be_bytes(time_bytes.try_into().map_err(|_| {
                    Error::Proof(ProofError::IncorrectValueSize(
                        "failed to convert timestamp bytes",
                    ))
                })?);

                let recipient = Identifier::from_bytes(&key).map_err(|_| {
                    Error::Proof(ProofError::IncorrectValueSize(
                        "failed to parse recipient identifier",
                    ))
                })?;

                let sum_item = match element {
                    Some(SumItem(value, ..)) if value >= 0 => value as TokenAmount,
                    Some(SumItem(..)) => {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "negative token amount in pre-programmed distribution".to_string(),
                        )));
                    }
                    _ => {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "proof element was not a sum item".to_string(),
                        )));
                    }
                };

                // Push to the vector for this timestamp
                acc.entry(time).or_default().push((recipient, sum_item));

                Ok(acc)
            },
        )?;

        // Convert grouped data into the final generic result structure `T`
        let result = grouped_data
            .into_iter()
            .map(|(time, recipients)| {
                let inner = recipients.into_iter().collect::<D>();
                (time, inner)
            })
            .collect::<T>();

        Ok((root_hash, result))
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
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::prelude::DataContract;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_pre_programmed_distributions() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");

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
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });

        let token_id = contract.token_id(0).expect("expected token at position 0");

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add an identity");

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // Create pre-programmed distributions
        let recipient_id = identity.id();
        let distributions = BTreeMap::from([
            (1000u64, BTreeMap::from([(recipient_id, 500u64)])),
            (2000u64, BTreeMap::from([(recipient_id, 1000u64)])),
        ]);

        let distribution = TokenPreProgrammedDistribution::V0(TokenPreProgrammedDistributionV0 {
            distributions: distributions.clone(),
        });

        let mut batch_operations = vec![];
        drive
            .add_pre_programmed_distributions(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                &distribution,
                &BlockInfo::default(),
                &mut None,
                &mut batch_operations,
                None,
                platform_version,
            )
            .expect("expected to add pre-programmed distributions");

        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                batch_operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to apply batch operations");

        // Generate proof
        let proof = drive
            .prove_token_pre_programmed_distributions(
                token_id.to_buffer(),
                None,
                None,
                None,
                platform_version,
            )
            .expect("expected to prove pre-programmed distributions");

        // Verify proof
        let (_, verified_distributions): (
            _,
            BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>,
        ) = Drive::verify_token_pre_programmed_distributions(
            proof.as_slice(),
            token_id.to_buffer(),
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        assert_eq!(verified_distributions, distributions);
    }

    #[test]
    fn should_prove_and_verify_empty_pre_programmed_distributions() {
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
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });

        let token_id = contract.token_id(0).expect("expected token at position 0");

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        // No distributions added -- prove and verify empty
        let proof = drive
            .prove_token_pre_programmed_distributions(
                token_id.to_buffer(),
                None,
                None,
                None,
                platform_version,
            )
            .expect("expected to prove pre-programmed distributions");

        let (_, verified_distributions): (
            _,
            BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>,
        ) = Drive::verify_token_pre_programmed_distributions(
            proof.as_slice(),
            token_id.to_buffer(),
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected proof verification to succeed");

        assert!(verified_distributions.is_empty());
    }
}
