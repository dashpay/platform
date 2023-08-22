use crate::types::{AssetLockProof, IdPublicKeyMap, Identity, IdentityPublicKey, MetaData};
use crate::{DppAssetLockProof, DppIdentity};
use std::{mem, slice};

pub(crate) fn build_c_identity_struct(maybe_identity: Option<DppIdentity>) -> *mut Identity {
    maybe_identity
        .map(|identity| {
            Box::into_raw(Box::from(Identity {
                protocol_version: identity.feature_version,
                id: Box::into_raw(Box::from(identity.id().0 .0)),
                public_keys_count: identity.public_keys().len(),
                public_keys: build_c_public_keys_struct(&identity),
                balance: identity.balance,
                revision: identity.revision,
                has_asset_lock_proof: identity.asset_lock_proof.is_some(),
                asset_lock_proof: build_c_asset_lock_proof_struct(&identity),
                has_metadata: identity.metadata.is_some(),
                meta_data: build_c_metadata_struct(&identity),
            }))
        })
        .unwrap_or(std::ptr::null_mut())
}

pub(crate) fn build_c_public_keys_struct(identity: &DppIdentity) -> *const *const IdPublicKeyMap {
    let mut id_public_key_map_as_vec: Vec<*const IdPublicKeyMap> = vec![];
    for (key_id, identity_public_key) in identity.public_keys() {
        id_public_key_map_as_vec.push(Box::into_raw(Box::from(IdPublicKeyMap {
            key: *key_id,
            public_key: Box::into_raw(Box::from(IdentityPublicKey {
                id: identity_public_key.id,
                purpose: identity_public_key.purpose as u8,
                security_level: identity_public_key.security_level as u8,
                key_type: identity_public_key.key_type as u8,
                read_only: identity_public_key.read_only,
                data_length: identity_public_key.data.len(),
                data: vec_to_pointer(identity_public_key.data.to_vec()),
                has_disabled_at: identity_public_key.disabled_at.is_some(),
                disabled_at: identity_public_key.disabled_at.unwrap_or(0),
            })),
        })))
    }
    let pointer = id_public_key_map_as_vec.as_ptr();
    mem::forget(id_public_key_map_as_vec);
    pointer
}

pub(crate) fn build_c_asset_lock_proof_struct(identity: &DppIdentity) -> *const AssetLockProof {
    let asset_lock_proof = &identity.asset_lock_proof;
    if let Some(asset_lock_proof) = asset_lock_proof {
        // TODO: construct the actual asset lock proofs
        match asset_lock_proof {
            DppAssetLockProof::Instant(..) => Box::into_raw(Box::from(AssetLockProof {
                is_chain: false,
                is_instant: true,
            })),
            DppAssetLockProof::Chain(..) => Box::into_raw(Box::from(AssetLockProof {
                is_chain: true,
                is_instant: false,
            })),
        }
    } else {
        Box::into_raw(Box::from(AssetLockProof {
            is_chain: false,
            is_instant: false,
        }))
    }
}

pub(crate) fn build_c_metadata_struct(identity: &DppIdentity) -> *const MetaData {
    let metadata = &identity.metadata;
    if let Some(metadata) = metadata {
        Box::into_raw(Box::from(MetaData {
            block_height: metadata.block_height,
            core_chain_locked_height: metadata.core_chain_locked_height,
            time_ms: metadata.time_ms,
            protocol_version: metadata.protocol_version,
        }))
    } else {
        std::ptr::null()
    }
}

pub(crate) fn extract_vector_from_pointer<T>(ptr: *const *const u8, count: usize) -> Vec<T> {
    let mut result = Vec::new();
    let inner_pointers = unsafe { slice::from_raw_parts(ptr, count) };
    for i in 0..count {
        let inner_item: T = unsafe { std::ptr::read(inner_pointers[i] as *const T) };
        result.push(inner_item);
    }
    result
}

pub(crate) fn vec_to_pointer<T>(a: Vec<T>) -> *const T {
    let ptr = a.as_ptr();
    mem::forget(a);
    ptr
}
