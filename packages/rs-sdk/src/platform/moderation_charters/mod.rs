//! The moderation charters system contract (protocol version 14): reading who moderates a
//! contract that declares elected moderation, and building the requests members send its
//! leader.
//!
//! Everything here is an ordinary proved document query on the system contract, through the
//! indexes its schema declares:
//!
//! * [`Sdk::fetch_seated_charter`]: the `electedCharter` whose `targetContractId` is the
//!   contract (`byTargetContract`). Only a contest's winner is ever stored there, contenders
//!   living in the contest, so there is at most one and it is the seated charter.
//! * [`Sdk::fetch_submitted_charter`]: a proposal by id, such as the seated charter's
//!   `submittedCharterId`.
//! * [`Sdk::fetch_moderation_team`]: the seated charter's team, the leader plus
//!   [`ElectedCharter::active_members`](dpp::moderation_charter::ElectedCharter::active_members)
//!   over its `addedModerator` and `removedModerator` documents (`byElectedCharterMember`).
//! * [`Sdk::fetch_submitted_charters`]: the proposals for a contract in filing order
//!   (`submittedCharter.byTargetContract`), one page at a time.
//! * [`Sdk::fetch_join_requests`]: the join requests for a proposal
//!   (`joinRequest.bySubmittedCharter`), one page at a time.
//! * [`Sdk::fetch_pending_resignation_requests`]: the resignation requests for a charter
//!   whose writer the leader has not removed yet.
//!
//! [`Sdk::build_join_request`] and [`Sdk::build_resignation_request`] build the two documents
//! whose message only the leader reads, encrypted with the generic
//! [`encrypted_for`](crate::platform::encrypted_for) helpers under the keys the schema's
//! `keyRequirements` demand: the leader's decryption key bound to `submittedCharter` and the
//! writer's encryption key bound to `joinRequest`.

mod readers;
mod requests;
mod team;

pub use readers::{CharterDocumentsPage, SeatedCharter};
pub use requests::{
    build_join_request_document, build_resignation_request_document, JoinRequestInput,
    ModerationCharterRequest, ResignationRequestInput,
};
pub use team::ModerationTeam;

use crate::platform::{DataContract, Fetch};
use crate::{Error, Sdk};
use dash_context_provider::ContextProvider;
use dpp::moderation_charter::MODERATION_CHARTERS_CONTRACT_ID;
use dpp::platform_value::string_encoding::Encoding;
use std::sync::Arc;

impl Sdk {
    /// The moderation charters system contract: from the context provider when it holds it,
    /// fetched otherwise.
    pub async fn fetch_moderation_charters_contract(&self) -> Result<Arc<DataContract>, Error> {
        if let Some(provider) = self.context_provider() {
            if let Some(contract) =
                provider.get_data_contract(&MODERATION_CHARTERS_CONTRACT_ID, self.version())?
            {
                return Ok(contract);
            }
        }
        DataContract::fetch(self, MODERATION_CHARTERS_CONTRACT_ID)
            .await?
            .map(Arc::new)
            .ok_or_else(|| {
                Error::MissingDependency(
                    "moderation charters contract".to_string(),
                    MODERATION_CHARTERS_CONTRACT_ID.to_string(Encoding::Base58),
                )
            })
    }
}
