use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::voting::resource_vote_choice::ResourceVoteChoiceWasm;
use crate::voting::vote_poll::VotePollWasm;
use dpp::voting::votes::Vote;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::yes_no_vote::YesNoVote;
use dpp::voting::votes::yes_no_vote::accessors::v0::YesNoVoteGettersV0;
use dpp::voting::votes::yes_no_vote::v0::YesNoVoteV0;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * A yes/no vote's answer, the same as a plain object and as JSON.
 */
export type YesNoAbstainVoteChoiceName = "yes" | "no" | "abstain";

/**
 * Vote serialized as a plain object.
 *
 * Internally tagged with `$type` ($-prefix because the level also carries
 * the inner vote's `$formatVersion`). Each variant flattens its V0 body — no
 * `data` wrapper. A yes/no vote names its poll by id only.
 */
export type VoteObject =
    | {
          $type: "resourceVote";
          $formatVersion: string;
          votePoll: VotePollObject;
          resourceVoteChoice: ResourceVoteChoiceObject;
      }
    | {
          $type: "yesNoVote";
          $formatVersion: string;
          votePollId: Uint8Array;
          voteChoice: YesNoAbstainVoteChoiceName;
      };

/**
 * Vote serialized as JSON.
 */
export type VoteJSON =
    | {
          $type: "resourceVote";
          $formatVersion: string;
          votePoll: VotePollJSON;
          resourceVoteChoice: ResourceVoteChoiceJSON;
      }
    | {
          $type: "yesNoVote";
          $formatVersion: string;
          votePollId: string;
          voteChoice: YesNoAbstainVoteChoiceName;
      };
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "VoteObject")]
    pub type VoteObjectJs;

    #[wasm_bindgen(typescript_type = "VoteJSON")]
    pub type VoteJSONJs;
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = "Vote")]
pub struct VoteWasm(Vote);

impl From<Vote> for VoteWasm {
    fn from(vote: Vote) -> Self {
        Self(vote)
    }
}

impl From<VoteWasm> for Vote {
    fn from(vote: VoteWasm) -> Self {
        vote.0
    }
}

#[wasm_bindgen(js_class = Vote)]
impl VoteWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(js_name = "votePoll")] vote_poll: &VotePollWasm,
        #[wasm_bindgen(js_name = "resourceVoteChoice")]
        resource_vote_choice: &ResourceVoteChoiceWasm,
    ) -> WasmDppResult<Self> {
        Ok(VoteWasm(Vote::ResourceVote(ResourceVote::V0(
            ResourceVoteV0::try_new(
                vote_poll.clone().into(),
                resource_vote_choice.clone().into(),
            )?,
        ))))
    }

    /// The poll a resource vote answers. A yes/no vote names its poll by id only: it has
    /// `pollId` instead.
    #[wasm_bindgen(getter = poll)]
    pub fn poll(&self) -> WasmDppResult<VotePollWasm> {
        match &self.0 {
            Vote::ResourceVote(vote) => Ok(vote.vote_poll().clone().into()),
            Vote::YesNoVote(_) => Err(names_its_poll_by_id()),
        }
    }

    /// The id of the poll a yes/no vote answers.
    #[wasm_bindgen(getter = "pollId")]
    pub fn poll_id(&self) -> WasmDppResult<IdentifierWasm> {
        match &self.0 {
            Vote::ResourceVote(_) => Err(not_a_yes_no_vote()),
            Vote::YesNoVote(vote) => Ok(vote.vote_poll_id().into()),
        }
    }

    /// The resource vote choice of a resource vote. A yes/no vote has `yesNoChoice` instead.
    #[wasm_bindgen(getter = choice)]
    pub fn choice(&self) -> WasmDppResult<ResourceVoteChoiceWasm> {
        match &self.0 {
            Vote::ResourceVote(vote) => Ok(vote.resource_vote_choice().into()),
            Vote::YesNoVote(_) => Err(not_a_resource_vote()),
        }
    }

    /// The answer of a yes/no vote: `yes`, `no` or `abstain`.
    #[wasm_bindgen(getter = "yesNoChoice")]
    pub fn yes_no_choice(&self) -> WasmDppResult<String> {
        match &self.0 {
            Vote::ResourceVote(_) => Err(not_a_yes_no_vote()),
            // The name the choice's serde writes, so it always matches `toJSON()`.
            Vote::YesNoVote(vote) => serde_json::to_value(vote.vote_choice())
                .map_err(|e| WasmDppError::serialization(e.to_string()))?
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| {
                    WasmDppError::serialization("a yes/no vote choice without a name".to_string())
                }),
        }
    }

    #[wasm_bindgen(setter = poll)]
    pub fn set_poll(&mut self, poll: &VotePollWasm) -> WasmDppResult<()> {
        self.0 = match self.0.clone() {
            Vote::ResourceVote(vote) => Vote::ResourceVote(ResourceVote::V0(
                ResourceVoteV0::try_new(poll.clone().into(), vote.resource_vote_choice())?,
            )),
            Vote::YesNoVote(_) => return Err(names_its_poll_by_id()),
        };

        Ok(())
    }

    #[wasm_bindgen(setter = "pollId")]
    pub fn set_poll_id(
        &mut self,
        #[wasm_bindgen(js_name = "pollId")] poll_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        let vote_poll_id = poll_id.try_into()?;
        self.0 = match self.0.clone() {
            Vote::ResourceVote(_) => return Err(not_a_yes_no_vote()),
            Vote::YesNoVote(vote) => Vote::YesNoVote(YesNoVote::V0(YesNoVoteV0 {
                vote_poll_id,
                vote_choice: vote.vote_choice(),
            })),
        };

        Ok(())
    }

    #[wasm_bindgen(setter = choice)]
    pub fn set_choice(&mut self, choice: &ResourceVoteChoiceWasm) -> WasmDppResult<()> {
        self.0 = match self.0.clone() {
            Vote::ResourceVote(vote) => Vote::ResourceVote(ResourceVote::V0(
                ResourceVoteV0::try_new(vote.vote_poll().clone(), choice.clone().into())?,
            )),
            Vote::YesNoVote(_) => return Err(not_a_resource_vote()),
        };

        Ok(())
    }
}

/// The resource vote choice exists on a resource vote only.
fn not_a_resource_vote() -> WasmDppError {
    WasmDppError::invalid_argument("this vote is a yes/no vote, not a resource vote".to_string())
}

/// The poll id and the yes/no answer exist on a yes/no vote only.
fn not_a_yes_no_vote() -> WasmDppError {
    WasmDppError::invalid_argument("this vote is a resource vote, not a yes/no vote".to_string())
}

/// A yes/no vote carries its poll's id, not the poll.
fn names_its_poll_by_id() -> WasmDppError {
    WasmDppError::invalid_argument(
        "a yes/no vote names its poll by id only: use pollId".to_string(),
    )
}

impl_try_from_js_value!(VoteWasm, "Vote");
impl_wasm_conversions_inner!(VoteWasm, Vote, Vote, VoteObjectJs, VoteJSONJs);
impl_wasm_type_info!(VoteWasm, Vote);
