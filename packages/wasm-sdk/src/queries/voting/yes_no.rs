//! Yes/no vote polls, served from protocol version 15: a poll's status and tallies
//! (`getYesNoVotePollState`) and a masternode's answer on a poll (`getMasternodeYesNoVote`).
//!
//! A poll is named by the `VotePoll` itself, as `getVotePollsByEndDate` returns it; the SDK
//! derives the poll's id from its purpose. The state returns the poll as it was opened, rule
//! included, so a caller can check it against the poll it asked about.

use crate::sdk::WasmSdk;
use crate::{ProofMetadataResponseWasm, WasmSdkError};
use dash_sdk::dpp::block::block_info::BlockInfo;
use dash_sdk::dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dash_sdk::dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStatus;
use dash_sdk::dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use dash_sdk::dpp::voting::vote_polls::VotePoll;
use dash_sdk::platform::yes_no_vote_polls::{
    MasternodeYesNoVote, MasternodeYesNoVoteQuery, YesNoVotePollState,
};
use dash_sdk::platform::Fetch;
use js_sys::{BigInt, Object, Reflect};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::IdentifierLikeJs;
use wasm_dpp2::VotePollWasm;

#[wasm_bindgen(typescript_custom_section)]
const YES_NO_VOTE_POLL_TS: &'static str = r#"
/** A block a yes/no vote poll opened or closed in. */
export interface YesNoVotePollBlock {
  height: bigint;
  coreHeight: number;
  timeMs: bigint;
  epoch: number;
}

/** The result a finished yes/no vote poll records. */
export interface YesNoVotePollResult {
  passed: boolean;
  yesVotingPower: number;
  noVotingPower: number;
  abstainVotingPower: number;
  /** The least yes plus no voting power the poll needed. */
  requiredVotingPower: number;
  startBlock: YesNoVotePollBlock;
  finalizationBlock: YesNoVotePollBlock;
}

/** The state of an opened yes/no vote poll. */
export interface YesNoVotePollState {
  /** The poll as it was opened: its purpose and the rule that decides it. */
  votePoll: VotePoll;
  /** 'started' while the poll is open, 'finished' once it closed. */
  status: 'started' | 'finished';
  /** The block the poll opened in; set while it is open. */
  startBlock?: YesNoVotePollBlock;
  /** The recorded result; set once the poll finished. */
  result?: YesNoVotePollResult;
  /** The voting power cast so far; zero once the poll finished and its votes were removed. */
  yesVotingPower: number;
  noVotingPower: number;
  abstainVotingPower: number;
}

/** A masternode's answer on a yes/no vote poll. */
export interface MasternodeYesNoVote {
  choice: YesNoAbstainVoteChoiceName;
  /** How many times the masternode has voted on the poll, its first vote included. */
  timesVoted: number;
}
"#;

/// Sets `key` on `object`.
fn set(object: &Object, key: &str, value: JsValue) -> Result<(), WasmSdkError> {
    Reflect::set(object, &key.into(), &value)
        .map(|_| ())
        .map_err(|_| WasmSdkError::generic(format!("failed to set `{key}`")))
}

/// The yes/no poll a `VotePoll` carries; a contested document resource poll is refused.
fn yes_no_vote_poll(vote_poll: &VotePollWasm) -> Result<YesNoVotePoll, WasmSdkError> {
    match VotePoll::from(vote_poll.clone()) {
        VotePoll::YesNoVotePoll(vote_poll) => Ok(vote_poll),
        VotePoll::ContestedDocumentResourceVotePoll(_) => Err(WasmSdkError::invalid_argument(
            "expected a yes/no vote poll, got a contested document resource vote poll",
        )),
    }
}

fn block_to_js(block: &BlockInfo) -> Result<JsValue, WasmSdkError> {
    let object = Object::new();
    set(&object, "height", BigInt::from(block.height).into())?;
    set(&object, "coreHeight", JsValue::from(block.core_height))?;
    set(&object, "timeMs", BigInt::from(block.time_ms).into())?;
    set(&object, "epoch", JsValue::from(block.epoch.index))?;
    Ok(object.into())
}

/// The state of an opened poll; `undefined` for a poll never opened.
fn state_to_js(state: Option<YesNoVotePollState>) -> Result<JsValue, WasmSdkError> {
    let Some(state) = state else {
        return Ok(JsValue::UNDEFINED);
    };
    let Some(stored_info) = state.stored_info else {
        return Ok(JsValue::UNDEFINED);
    };
    let object = Object::new();
    set(
        &object,
        "votePoll",
        VotePollWasm::from(VotePoll::YesNoVotePoll(stored_info.vote_poll().clone())).into(),
    )?;
    match stored_info.status() {
        YesNoVotePollStatus::Started(start_block) => {
            set(&object, "status", JsValue::from_str("started"))?;
            set(&object, "startBlock", block_to_js(start_block)?)?;
        }
        YesNoVotePollStatus::Finished(finished) => {
            set(&object, "status", JsValue::from_str("finished"))?;
            let result = Object::new();
            set(&result, "passed", JsValue::from(finished.passed))?;
            set(
                &result,
                "yesVotingPower",
                JsValue::from(finished.yes_voting_power),
            )?;
            set(
                &result,
                "noVotingPower",
                JsValue::from(finished.no_voting_power),
            )?;
            set(
                &result,
                "abstainVotingPower",
                JsValue::from(finished.abstain_voting_power),
            )?;
            set(
                &result,
                "requiredVotingPower",
                JsValue::from(finished.required_voting_power),
            )?;
            set(&result, "startBlock", block_to_js(&finished.start_block)?)?;
            set(
                &result,
                "finalizationBlock",
                block_to_js(&finished.finalization_block)?,
            )?;
            set(&object, "result", result.into())?;
        }
    }
    set(
        &object,
        "yesVotingPower",
        JsValue::from(state.yes_voting_power),
    )?;
    set(
        &object,
        "noVotingPower",
        JsValue::from(state.no_voting_power),
    )?;
    set(
        &object,
        "abstainVotingPower",
        JsValue::from(state.abstain_voting_power),
    )?;
    Ok(object.into())
}

/// A masternode's answer; `undefined` when it has not voted on the poll.
fn vote_to_js(vote: Option<MasternodeYesNoVote>) -> Result<JsValue, WasmSdkError> {
    let Some(vote) = vote else {
        return Ok(JsValue::UNDEFINED);
    };
    let object = Object::new();
    let choice = match vote.vote_choice {
        YesNoAbstainVoteChoice::Yes => "yes",
        YesNoAbstainVoteChoice::No => "no",
        YesNoAbstainVoteChoice::Abstain => "abstain",
    };
    set(&object, "choice", JsValue::from_str(choice))?;
    set(&object, "timesVoted", JsValue::from(vote.times_voted))?;
    Ok(object.into())
}

#[wasm_bindgen]
impl WasmSdk {
    /// The status and tallies of a yes/no vote poll: started, with the voting power cast so
    /// far, or finished, with the result it records. `undefined` for a poll never opened.
    /// Networks serve it from protocol version 15.
    ///
    /// # Example
    /// ```javascript
    /// const state = await sdk.getYesNoVotePollState(votePoll);
    /// if (state?.status === 'finished') console.log(state.result.passed);
    /// ```
    #[wasm_bindgen(
        js_name = "getYesNoVotePollState",
        unchecked_return_type = "YesNoVotePollState | undefined"
    )]
    pub async fn get_yes_no_vote_poll_state(
        &self,
        #[wasm_bindgen(js_name = "votePoll")] vote_poll: &VotePollWasm,
    ) -> Result<JsValue, WasmSdkError> {
        let vote_poll = yes_no_vote_poll(vote_poll)?;
        let state = YesNoVotePollState::fetch(self.as_ref(), vote_poll).await?;
        state_to_js(state)
    }

    /// The state of a yes/no vote poll together with its proof and metadata.
    #[wasm_bindgen(
        js_name = "getYesNoVotePollStateWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<YesNoVotePollState | undefined>"
    )]
    pub async fn get_yes_no_vote_poll_state_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "votePoll")] vote_poll: &VotePollWasm,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let vote_poll = yes_no_vote_poll(vote_poll)?;
        let (state, metadata, proof) =
            YesNoVotePollState::fetch_with_metadata_and_proof(self.as_ref(), vote_poll, None)
                .await?;
        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            state_to_js(state)?,
            metadata,
            proof,
        ))
    }

    /// A masternode's answer on a yes/no vote poll and how many times it has voted on it.
    /// `undefined` when the masternode has not voted on the poll, and once the poll finished,
    /// which removes its votes. Networks serve it from protocol version 15.
    ///
    /// # Example
    /// ```javascript
    /// const vote = await sdk.getMasternodeYesNoVote(proTxHash, votePoll);
    /// console.log(vote?.choice);
    /// ```
    #[wasm_bindgen(
        js_name = "getMasternodeYesNoVote",
        unchecked_return_type = "MasternodeYesNoVote | undefined"
    )]
    pub async fn get_masternode_yes_no_vote(
        &self,
        #[wasm_bindgen(js_name = "proTxHash")] pro_tx_hash: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "votePoll")] vote_poll: &VotePollWasm,
    ) -> Result<JsValue, WasmSdkError> {
        let query = masternode_yes_no_vote_query(pro_tx_hash, vote_poll)?;
        let vote = MasternodeYesNoVote::fetch(self.as_ref(), query).await?;
        vote_to_js(vote)
    }

    /// A masternode's answer on a yes/no vote poll together with its proof and metadata.
    #[wasm_bindgen(
        js_name = "getMasternodeYesNoVoteWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<MasternodeYesNoVote | undefined>"
    )]
    pub async fn get_masternode_yes_no_vote_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "proTxHash")] pro_tx_hash: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "votePoll")] vote_poll: &VotePollWasm,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query = masternode_yes_no_vote_query(pro_tx_hash, vote_poll)?;
        let (vote, metadata, proof) =
            MasternodeYesNoVote::fetch_with_metadata_and_proof(self.as_ref(), query, None).await?;
        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            vote_to_js(vote)?,
            metadata,
            proof,
        ))
    }
}

/// The query for a masternode's answer on a yes/no poll. It carries the poll itself, so the
/// SDK derives its id with the version it speaks once it knows the network serves the query.
fn masternode_yes_no_vote_query(
    pro_tx_hash: IdentifierLikeJs,
    vote_poll: &VotePollWasm,
) -> Result<MasternodeYesNoVoteQuery, WasmSdkError> {
    let pro_tx_hash = pro_tx_hash
        .try_into()
        .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid proTxHash: {err}")))?;
    Ok(MasternodeYesNoVoteQuery::new(
        pro_tx_hash,
        yes_no_vote_poll(vote_poll)?,
    ))
}
