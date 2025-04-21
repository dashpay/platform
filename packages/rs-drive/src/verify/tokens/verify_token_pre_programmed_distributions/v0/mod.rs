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
