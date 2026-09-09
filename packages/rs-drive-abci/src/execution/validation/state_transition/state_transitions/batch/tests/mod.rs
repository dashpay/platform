mod document;
mod token;

/// Sets valid DIP-33 payment addresses on a random profile document when the
/// schema version carries the fields (dashpay v2, protocol version 14+): the
/// generator fills them with random bytes, and the data trigger requires a
/// valid leading type byte (0x00 P2PKH / 0x01 P2SH).
pub(crate) fn set_valid_profile_payment_addresses(
    document: &mut dpp::document::Document,
    profile: dpp::data_contract::document_type::DocumentTypeRef,
) {
    if profile.properties().contains_key("corePaymentAddress") {
        document.set("corePaymentAddress", vec![0u8; 21].into());
        let mut platform_payment_address = vec![1u8];
        platform_payment_address.extend([0u8; 20]);
        document.set("platformPaymentAddress", platform_payment_address.into());
    }
}

use crate::platform_types::platform_state::PlatformStateV0Methods;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};

use dpp::document::document_methods::DocumentMethodsV0;

use dpp::document::{DocumentV0Getters, DocumentV0Setters};

use dpp::identity::accessors::IdentityGettersV0;

use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;

use dpp::serialization::PlatformSerializable;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;

use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::drive::document::query::QueryDocumentsWithFlagsOutcomeV0Methods;

use crate::execution::validation::state_transition::tests::add_tokens_to_identity;
use crate::execution::validation::state_transition::tests::process_test_state_transition;
use crate::execution::validation::state_transition::tests::setup_identity;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::PaidConsensusError;
use crate::test::helpers::setup::TestPlatformBuilder;
use assert_matches::assert_matches;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::document::transfer::Transferable;
use dpp::fee::fee_result::BalanceChange;
use dpp::fee::Credits;
use dpp::nft::TradeMode;
use dpp::platform_value::{Bytes32, Value};
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use drive::query::DriveDocumentQuery;
use drive::util::storage_flags::StorageFlags;
use rand::prelude::StdRng;
use rand::Rng;
use rand::SeedableRng;
