use crate::platform_types::signature_verification_quorum_set::v0::quorum_config_for_saving_v0::QuorumConfigForSavingV0;
use crate::platform_types::signature_verification_quorum_set::v0::quorum_set::PreviousPastQuorumsV0;
use crate::platform_types::signature_verification_quorum_set::{
    Quorums, SignatureVerificationQuorumSetForSaving, SignatureVerificationQuorumSetV0,
    VerificationQuorum,
};
use bincode::{Decode, Encode};
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
use dpp::platform_value::Bytes32;

#[derive(Debug, Clone, Encode, Decode)]
pub struct SignatureVerificationQuorumSetForSavingV0 {
    config: QuorumConfigForSavingV0,
    current_quorums: Vec<QuorumForSavingV0>,
    previous_quorums: Option<PreviousPastQuorumsForSavingV0>,
}

impl From<SignatureVerificationQuorumSetForSavingV0> for SignatureVerificationQuorumSetForSaving {
    fn from(value: SignatureVerificationQuorumSetForSavingV0) -> Self {
        SignatureVerificationQuorumSetForSaving::V0(value)
    }
}

impl From<SignatureVerificationQuorumSetV0> for SignatureVerificationQuorumSetForSavingV0 {
    fn from(value: SignatureVerificationQuorumSetV0) -> Self {
        let SignatureVerificationQuorumSetV0 {
            config,
            current_quorums,
            previous,
        } = value;

        Self {
            config: config.into(),
            current_quorums: current_quorums.into(),
            previous_quorums: previous.map(|previous| previous.into()),
        }
    }
}

impl From<SignatureVerificationQuorumSetForSavingV0> for SignatureVerificationQuorumSetV0 {
    fn from(value: SignatureVerificationQuorumSetForSavingV0) -> Self {
        let SignatureVerificationQuorumSetForSavingV0 {
            config,
            current_quorums,
            previous_quorums,
        } = value;

        Self {
            config: config.into(),
            current_quorums: current_quorums.into(),
            previous: previous_quorums.map(|previous| previous.into()),
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct PreviousPastQuorumsForSavingV0 {
    quorums: Vec<QuorumForSavingV0>,
    last_active_core_height: u32,
    updated_at_core_height: u32,
    previous_change_height: Option<u32>,
}

impl From<PreviousPastQuorumsV0> for PreviousPastQuorumsForSavingV0 {
    fn from(value: PreviousPastQuorumsV0) -> Self {
        let PreviousPastQuorumsV0 {
            quorums,
            last_active_core_height,
            updated_at_core_height,
            previous_change_height,
        } = value;

        Self {
            quorums: quorums.into(),
            last_active_core_height,
            updated_at_core_height,
            previous_change_height,
        }
    }
}

impl From<PreviousPastQuorumsForSavingV0> for PreviousPastQuorumsV0 {
    fn from(value: PreviousPastQuorumsForSavingV0) -> Self {
        let PreviousPastQuorumsForSavingV0 {
            quorums,
            last_active_core_height: active_core_height,
            updated_at_core_height,
            previous_change_height,
        } = value;

        Self {
            quorums: quorums.into(),
            last_active_core_height: active_core_height,
            updated_at_core_height,
            previous_change_height,
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct QuorumForSavingV0 {
    hash: Bytes32,
    #[cfg(feature = "bls-signatures")]
    #[bincode(with_serde)]
    public_key: bls_signatures::PublicKey,
    index: Option<u32>,
}

impl From<Vec<QuorumForSavingV0>> for Quorums<VerificationQuorum> {
    fn from(value: Vec<QuorumForSavingV0>) -> Self {
        Quorums::from_iter(value.into_iter().map(|quorum| {
            (
                QuorumHash::from_byte_array(quorum.hash.to_buffer()),
                VerificationQuorum {
                    #[cfg(feature = "bls-signatures")]
                    public_key: dpp::bls::PublicKey::try_from(
                        quorum.public_key.to_bytes().as_slice(),
                    )
                    .expect("expected to convert between BLS key libraries (from chia)"),
                    #[cfg(not(feature = "bls-signatures"))]
                    public_key: Default::default(),
                    index: quorum.index,
                },
            )
        }))
    }
}

impl From<Quorums<VerificationQuorum>> for Vec<QuorumForSavingV0> {
    fn from(quorums: Quorums<VerificationQuorum>) -> Self {
        quorums
            .into_iter()
            .map(|(hash, quorum)| QuorumForSavingV0 {
                hash: Bytes32::from(hash.as_byte_array()),
                #[cfg(feature = "bls-signatures")]
                public_key: bls_signatures::PublicKey::from_bytes(&quorum.public_key.to_bytes())
                    .expect("expected to convert between BLS key libraries (to chia)"),
                index: quorum.index,
            })
            .collect()
    }
}

#[cfg(all(test, feature = "bls-signatures"))]
mod tests {
    use super::*;
    use crate::platform_types::signature_verification_quorum_set::SignatureVerificationQuorumSet;
    use dpp::dashcore_rpc::json::QuorumType;

    fn assert_quorum(
        quorums: &Quorums<VerificationQuorum>,
        hash: u8,
        public_key: &str,
        index: Option<u32>,
    ) {
        let quorum = quorums
            .get(&QuorumHash::from_byte_array([hash; 32]))
            .unwrap();
        assert_eq!(
            quorum.public_key.to_bytes().as_slice(),
            hex::decode(public_key).unwrap()
        );
        assert_eq!(quorum.index, index);
    }

    #[test]
    fn should_restore_current_and_previous_quorums_from_frozen_v0_storage() {
        let bytes = hex::decode(super::super::storage_vectors::QUORUM_STORAGE_V0.trim()).unwrap();
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let (stored, consumed): (SignatureVerificationQuorumSetForSaving, _) =
            bincode::decode_from_slice(&bytes, config).unwrap();
        assert_eq!(consumed, bytes.len());
        assert!(matches!(
            stored,
            SignatureVerificationQuorumSetForSaving::V0(_)
        ));
        assert_eq!(bincode::encode_to_vec(&stored, config).unwrap(), bytes);

        let SignatureVerificationQuorumSet::V0(restored) = stored.into();
        assert_eq!(restored.config.quorum_type, QuorumType::Llmq400_60);
        assert_eq!(restored.config.active_signers, 4);
        assert!(!restored.config.rotation);
        assert_eq!(restored.config.window, 288);
        assert_eq!(restored.current_quorums.len(), 2);
        assert_quorum(
            &restored.current_quorums,
            0x11,
            "97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
            None,
        );
        assert_quorum(
            &restored.current_quorums,
            0x22,
            "a572cbea904d67468808c8eb50a9450c9721db309128012543902d0ac358a62ae28f75bb8f1c7c42c39a8c5529bf0f4e",
            Some(0),
        );

        let previous = restored.previous.unwrap();
        assert_eq!(previous.last_active_core_height, 1000);
        assert_eq!(previous.updated_at_core_height, 1008);
        assert_eq!(previous.previous_change_height, Some(900));
        assert_eq!(previous.quorums.len(), 2);
        assert_quorum(
            &previous.quorums,
            0x33,
            "95fde78acd5f6886ddaf5d0056610167c513d09c1c0efabbc7cdcc69beea113779c4a81e2d24daafc5387dbf6ac5fe48",
            Some(1),
        );
        assert_quorum(
            &previous.quorums,
            0x44,
            "b7f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb",
            None,
        );
    }
}
