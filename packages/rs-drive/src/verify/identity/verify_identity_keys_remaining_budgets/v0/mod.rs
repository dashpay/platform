use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::fee::Credits;
use dpp::identity::KeyID;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::GroveDb;
use integer_encoding::VarInt;

impl Drive {
    #[inline(always)]
    pub(super) fn verify_identity_keys_remaining_budgets_v0<
        T: FromIterator<(KeyID, Option<Credits>)>,
    >(
        proof: &[u8],
        identity_id: [u8; 32],
        key_ids: &[KeyID],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::identity_keys_remaining_budgets_query(identity_id, key_ids);
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
        if proved_key_values.len() != key_ids.len() {
            return Err(Error::Proof(ProofError::WrongElementCount {
                expected: key_ids.len(),
                got: proved_key_values.len(),
            }));
        }
        let values = proved_key_values
            .into_iter()
            .map(|(_, key, element)| {
                let (key_id, _) = KeyID::decode_var(key.as_slice()).ok_or(Error::Proof(
                    ProofError::IncorrectValueSize("the key of a budget entry is not a key id"),
                ))?;
                match element {
                    Some(Item(encoded_budget, _)) => {
                        let remaining_budget = Credits::from_be_bytes(
                            encoded_budget.as_slice().try_into().map_err(|_| {
                                Error::Proof(ProofError::IncorrectValueSize(
                                    "the remaining budget of a key must be 8 bytes",
                                ))
                            })?,
                        );
                        Ok((key_id, Some(remaining_budget)))
                    }
                    None => Ok((key_id, None)),
                    Some(_) => Err(Error::Proof(ProofError::IncorrectValueSize(
                        "the proof did not point to a remaining budget item",
                    ))),
                }
            })
            .collect::<Result<T, Error>>()?;
        Ok((root_hash, values))
    }
}
