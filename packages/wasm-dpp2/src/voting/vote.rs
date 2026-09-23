use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::voting::resource_vote_choice::ResourceVoteChoiceWasm;
use crate::voting::vote_poll::VotePollWasm;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::VotePoll;
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
 * Vote serialized as a plain object.
 *
 * Internally tagged with `$type` ($-prefix because the level also carries
 * the inner ResourceVote's `$formatVersion`). The single ResourceVote
 * variant flattens its V0 body — no `data` wrapper.
 */
export interface VoteObject {
    $type: "resourceVote";
    $formatVersion: string;
    votePoll: VotePollObject;
    resourceVoteChoice: ResourceVoteChoiceObject;
}

/**
 * Vote serialized as JSON.
 */
export interface VoteJSON {
    $type: "resourceVote";
    $formatVersion: string;
    votePoll: VotePollJSON;
    resourceVoteChoice: ResourceVoteChoiceJSON;
}
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
    ) -> Self {
        VoteWasm(Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
            vote_poll: vote_poll.clone().into(),
            resource_vote_choice: resource_vote_choice.clone().into(),
        })))
    }

    #[wasm_bindgen(getter = poll)]
    pub fn poll(&self) -> VotePollWasm {
        match &self.0 {
            Vote::ResourceVote(vote) => vote.vote_poll().clone().into(),
            Vote::YesNoVote(vote) => VotePoll::YesNoVotePoll(vote.vote_poll().clone()).into(),
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
            Vote::ResourceVote(_) => Err(WasmDppError::invalid_argument(
                "this vote is a resource vote, not a yes/no vote".to_string(),
            )),
            Vote::YesNoVote(vote) => Ok(match vote.vote_choice() {
                YesNoAbstainVoteChoice::Yes => "yes".to_string(),
                YesNoAbstainVoteChoice::No => "no".to_string(),
                YesNoAbstainVoteChoice::Abstain => "abstain".to_string(),
            }),
        }
    }

    #[wasm_bindgen(setter = poll)]
    pub fn set_poll(&mut self, poll: &VotePollWasm) -> WasmDppResult<()> {
        self.0 = match self.0.clone() {
            Vote::ResourceVote(vote) => Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: poll.clone().into(),
                resource_vote_choice: vote.resource_vote_choice(),
            })),
            Vote::YesNoVote(vote) => {
                let VotePoll::YesNoVotePoll(vote_poll) = poll.clone().into() else {
                    return Err(WasmDppError::invalid_argument(
                        "a yes/no vote answers a yes/no vote poll".to_string(),
                    ));
                };
                Vote::YesNoVote(YesNoVote::V0(YesNoVoteV0 {
                    vote_poll,
                    vote_choice: vote.vote_choice(),
                }))
            }
        };

        Ok(())
    }

    #[wasm_bindgen(setter = choice)]
    pub fn set_choice(&mut self, choice: &ResourceVoteChoiceWasm) -> WasmDppResult<()> {
        self.0 = match self.0.clone() {
            Vote::ResourceVote(vote) => Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: vote.vote_poll().clone(),
                resource_vote_choice: choice.clone().into(),
            })),
            Vote::YesNoVote(_) => return Err(not_a_resource_vote()),
        };

        Ok(())
    }
}

/// The resource vote choice exists on a resource vote only.
fn not_a_resource_vote() -> WasmDppError {
    WasmDppError::invalid_argument("this vote is a yes/no vote, not a resource vote".to_string())
}

impl_try_from_js_value!(VoteWasm, "Vote");
impl_wasm_conversions_inner!(VoteWasm, Vote, Vote, VoteObjectJs, VoteJSONJs);
impl_wasm_type_info!(VoteWasm, Vote);
