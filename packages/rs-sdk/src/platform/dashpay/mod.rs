//! DashPay contact request and invitation helpers
//!
//! This module provides helper functions for creating and sending DashPay contact requests
//! according to DIP-15 specification, for creating, updating and fetching DashPay profiles,
//! the DashPay invitation link codec and claim, and private contact requests (protocol
//! version 15).

mod contact_request;
mod contact_request_queries;
mod invitation;
mod private_contact_request;
mod profile;

pub use contact_request::{
    recipient_key_purpose_is_acceptable_on_receive, recipient_key_purpose_is_valid,
    sender_key_purpose_is_acceptable_on_receive, ContactRequestInput, ContactRequestResult,
    EcdhProvider, RecipientIdentity, SendContactRequestInput, SendContactRequestResult,
};
pub use contact_request_queries::{ContactRequestDocuments, ResumableContactRequests};
pub use invitation::{
    encode_invitation_uri, parse_invitation_uri, voucher_credit_output, voucher_output_index,
    wif_network_matches, InvitationClaimStatus, InvitationError, InviterInfo, ParsedInvitation,
    MAX_INVITATION_DUFFS, MAX_INVITATION_TTL_SECS, MIN_INVITATION_DUFFS,
};
pub use private_contact_request::{
    private_contact_request_outgoing_viewing_key, private_contact_request_senders,
    private_contact_requests_from_scan, seal_private_contact_request, PrivateContactRequest,
    PrivateContactRequestRejection, PrivateContactRequestScan, PrivateContactRequestSender,
    ReceivedPrivateContactRequest, RejectedPrivateContactRequest, SentPrivateContactRequest,
    PRIVATE_CONTACT_REQUEST_ACCOUNT_LABEL_MAX_BYTES,
    PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MAX_LEN,
    PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MIN_LEN, PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
    PRIVATE_CONTACT_REQUEST_SHIELDED_ADDRESS_LEN,
};
pub use profile::ProfileFields;

use crate::{Error, Sdk};
use dpp::prelude::Identifier;
use std::sync::Arc;

impl Sdk {
    /// Helper method to get the DashPay contract ID
    fn get_dashpay_contract_id(&self) -> Result<Identifier, Error> {
        // Get DashPay contract ID from system contract if available
        #[cfg(feature = "dashpay-contract")]
        let dashpay_contract_id = {
            use dpp::system_data_contracts::SystemDataContract;
            SystemDataContract::Dashpay.id()
        };

        #[cfg(not(feature = "dashpay-contract"))]
        let dashpay_contract_id = {
            // The deployed DashPay v1 contract id. This fallback
            // previously held the DPNS id — a latent foot-gun for
            // builds without the `dashpay-contract` feature.
            const DASHPAY_CONTRACT_ID: &str = "Bwr4WHCPz5rFVAD87RqTs3izo4zpzwsEdKPWUT1NS1C7";
            Identifier::from_string(
                DASHPAY_CONTRACT_ID,
                dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| Error::Generic(format!("Invalid DashPay contract ID: {}", e)))?
        };

        Ok(dashpay_contract_id)
    }

    /// Helper method to fetch the DashPay contract, checking context provider first
    async fn fetch_dashpay_contract(&self) -> Result<Arc<dpp::data_contract::DataContract>, Error> {
        let dashpay_contract_id = self.get_dashpay_contract_id()?;
        self.fetch_system_data_contract(dashpay_contract_id)
            .await?
            .ok_or_else(|| Error::Generic("DashPay contract not found".to_string()))
    }
}
