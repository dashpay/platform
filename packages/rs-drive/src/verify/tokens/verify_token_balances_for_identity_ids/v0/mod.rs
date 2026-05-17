use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::balances::credits::TokenAmount;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_token_balances_for_identity_ids_v0<
        T: FromIterator<(I, Option<TokenAmount>)>,
        I: From<[u8; 32]>,
    >(
        proof: &[u8],
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::token_balances_for_identity_ids_query(token_id, identity_ids);
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
        if proved_key_values.len() == identity_ids.len() {
            let values = proved_key_values
                .into_iter()
                .map(|proved_key_value| {
                    let key: [u8; 32] = proved_key_value
                        .1
                        .try_into()
                        .map_err(|_| Error::Proof(ProofError::IncorrectValueSize("value size")))?;
                    let maybe_element = proved_key_value.2;
                    match maybe_element {
                        None => Ok((key.into(), None)),
                        Some(element) => {
                            let balance: TokenAmount = element
                                .as_sum_item_value()
                                .map_err(Error::from)?
                                .try_into()
                                .map_err(|_| {
                                    Error::Proof(ProofError::IncorrectValueSize(
                                        "balance was negative",
                                    ))
                                })?;
                            Ok((key.into(), Some(balance)))
                        }
                    }
                })
                .collect::<Result<T, Error>>()?;
            Ok((root_hash, values))
        } else {
            Err(Error::Proof(ProofError::WrongElementCount {
                expected: identity_ids.len(),
                got: proved_key_values.len(),
            }))
        }
    }
}
