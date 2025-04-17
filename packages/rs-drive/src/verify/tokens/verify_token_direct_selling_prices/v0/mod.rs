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
