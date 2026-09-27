use bincode::{Decode, Encode};

pub mod v1;
pub mod v2;
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct VoteResolutionFundFees {
    /// This is the amount that will be deducted from an identity and used to pay for voting
    pub contested_document_vote_resolution_fund_required_amount: u64,
    /// This is the amount that will be deducted from an identity and used to pay for voting if we are currently locked
    pub contested_document_vote_resolution_unlock_fund_required_amount: u64,
    /// This is the amount that a single vote will cost
    pub contested_document_single_vote_cost: u64,
    /// The amount deducted from an identity that applies in a moderation election (an
    /// `electedCharter` create on the moderation charters contract), prefunding the
    /// masternode votes; what is left when the poll ends is released as processing fees.
    /// Shared by the application, challenge and amendment polls. Moderation elections exist
    /// from protocol version 14; every earlier schedule carries the contested document
    /// amount here, so choosing between the two changes nothing before 14.
    pub moderation_vote_resolution_fund_required_amount: u64,
}

/// The vote resolution fund fees exactly as every pre-1.4 release serialized them inside
/// the platform state's fee versions (3 fields). `VoteResolutionFundFees` gained
/// `moderation_vote_resolution_fund_required_amount` in 4.2; embedding the live struct in
/// `FeeVersionFieldsBeforeVersion4` would shift that frozen wire format. Any future field
/// added to `VoteResolutionFundFees` must NOT be added here.
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct VoteResolutionFundFeesFieldsBeforeVersion4 {
    pub contested_document_vote_resolution_fund_required_amount: u64,
    pub contested_document_vote_resolution_unlock_fund_required_amount: u64,
    pub contested_document_single_vote_cost: u64,
}

impl From<VoteResolutionFundFeesFieldsBeforeVersion4> for VoteResolutionFundFees {
    fn from(value: VoteResolutionFundFeesFieldsBeforeVersion4) -> Self {
        VoteResolutionFundFees {
            contested_document_vote_resolution_fund_required_amount: value
                .contested_document_vote_resolution_fund_required_amount,
            contested_document_vote_resolution_unlock_fund_required_amount: value
                .contested_document_vote_resolution_unlock_fund_required_amount,
            contested_document_single_vote_cost: value.contested_document_single_vote_cost,
            // Pre-4.2 tables predate moderation elections, which pay the contested document
            // amount wherever they could be reached before protocol version 14.
            moderation_vote_resolution_fund_required_amount: value
                .contested_document_vote_resolution_fund_required_amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{VoteResolutionFundFees, VoteResolutionFundFeesFieldsBeforeVersion4};
    use crate::version::fee::vote_resolution_fund_fees::v1::VOTE_RESOLUTION_FUND_FEES_VERSION1;

    #[test]
    // If this test failed, then a new field was added in VoteResolutionFundFees. And the corresponding eq needs to be updated as well
    fn test_fee_storage_version_equality() {
        let version1 = VoteResolutionFundFees {
            contested_document_vote_resolution_fund_required_amount: 1,
            contested_document_vote_resolution_unlock_fund_required_amount: 2,
            contested_document_single_vote_cost: 3,
            moderation_vote_resolution_fund_required_amount: 4,
        };

        let version2 = VoteResolutionFundFees {
            contested_document_vote_resolution_fund_required_amount: 1,
            contested_document_vote_resolution_unlock_fund_required_amount: 2,
            contested_document_single_vote_cost: 3,
            moderation_vote_resolution_fund_required_amount: 4,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "VoteResolutionFundFees equality test failed. If a field was added or removed, update the Eq implementation.");
    }

    /// A pre-1.4 platform state stored exactly three amounts, in this order. Their encoding
    /// is built here from a plain tuple, not from the frozen struct, so a field added to or
    /// reordered in the struct fails to decode it or decodes it wrongly.
    #[test]
    fn should_decode_the_three_field_encoding_of_pre_1_4_platform_states() {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let stored: (u64, u64, u64) = (20_000_000_000, 400_000_000_000, 10_000_000);
        let bytes = bincode::encode_to_vec(stored, config).expect("encodes");

        let (decoded, read): (VoteResolutionFundFeesFieldsBeforeVersion4, usize) =
            bincode::decode_from_slice(&bytes, config).expect("decodes");
        assert_eq!(read, bytes.len(), "no trailing bytes are expected");
        assert_eq!(
            decoded,
            VoteResolutionFundFeesFieldsBeforeVersion4 {
                contested_document_vote_resolution_fund_required_amount: 20_000_000_000,
                contested_document_vote_resolution_unlock_fund_required_amount: 400_000_000_000,
                contested_document_single_vote_cost: 10_000_000,
            }
        );
        assert_eq!(
            bincode::encode_to_vec(&decoded, config).expect("encodes"),
            bytes,
            "the frozen struct encodes as the three amounts and nothing else"
        );
        assert_eq!(
            VoteResolutionFundFees::from(decoded),
            VOTE_RESOLUTION_FUND_FEES_VERSION1
        );
    }
}
