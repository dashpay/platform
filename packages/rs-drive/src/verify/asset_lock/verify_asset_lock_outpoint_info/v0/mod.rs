use crate::drive::{Drive, RootTree};
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::platform_value::Bytes36;
use dpp::serialization::PlatformDeserializableTrusted;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb, PathQuery};

fn asset_lock_outpoint_info_query(outpoint: &[u8; 36]) -> PathQuery {
    let mut query = PathQuery::new_single_key(
        vec![vec![RootTree::SpentAssetLockTransactions as u8]],
        outpoint.to_vec(),
    );
    query.query.limit = Some(1);
    query
}

impl Drive {
    pub(super) fn verify_asset_lock_outpoint_info_v0(
        proof: &[u8],
        outpoint: &Bytes36,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StoredAssetLockInfo), Error> {
        let (root_hash, mut values) = GroveDb::verify_query_with_absence_proof(
            proof,
            &asset_lock_outpoint_info_query(&outpoint.0),
            &platform_version.drive.grove_version,
        )?;
        if values.len() != 1 {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "asset-lock proof must contain one result".to_string(),
            )));
        }
        let info = match values.remove(0).2 {
            None => StoredAssetLockInfo::NotPresent,
            Some(element) => {
                let bytes = match element {
                    Element::Item(bytes, _) => bytes,
                    _ => {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "asset-lock proof must contain an item".to_string(),
                        )))
                    }
                };
                if bytes.is_empty() {
                    StoredAssetLockInfo::FullyConsumed
                } else {
                    StoredAssetLockInfo::PartiallyConsumed(
                        AssetLockValue::deserialize_from_bytes_trusted(&bytes)?,
                    )
                }
            }
        };
        Ok((root_hash, info))
    }
}
