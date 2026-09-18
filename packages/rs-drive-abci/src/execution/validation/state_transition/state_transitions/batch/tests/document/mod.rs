mod creation;
mod deletion;
mod dpns;
mod immutable;
mod index_only;
mod keep_history;
mod nft;
mod ranked_group_drain;
mod replacement;
mod required_since;
mod system_agreement;
mod transfer;

use super::*;

use crate::execution::validation::state_transition::tests::create_card_game_internal_token_contract_with_owner_identity_burn_tokens;
