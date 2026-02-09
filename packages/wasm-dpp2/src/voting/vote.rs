use crate::impl_try_from_js_value;
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::voting::resource_vote_choice::ResourceVoteChoiceWasm;
use crate::voting::vote_poll::VotePollWasm;
use dpp::voting::votes::Vote;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Vote serialized as a plain object.
 */
export interface VoteObject {
    resourceVote: {
        $version: string;
        votePoll: VotePollObject;
        resourceVoteChoice: ResourceVoteChoiceObject;
    };
}

/**
 * Vote serialized as JSON.
 */
export interface VoteJSON {
    resourceVote: {
        $version: string;
        votePoll: VotePollJSON;
        resourceVoteChoice: ResourceVoteChoiceJSON;
    };
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "VoteObject")]
    pub type VoteObjectJs;

    #[wasm_bindgen(typescript_type = "VoteJSON")]
    pub type VoteJSONJs;
}

#[derive(Clone)]
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
        }
    }

    #[wasm_bindgen(getter = choice)]
    pub fn choice(&self) -> ResourceVoteChoiceWasm {
        match &self.0 {
            Vote::ResourceVote(vote) => vote.resource_vote_choice().into(),
        }
    }

    #[wasm_bindgen(setter = poll)]
    pub fn set_poll(&mut self, poll: &VotePollWasm) {
        self.0 = match self.0.clone() {
            Vote::ResourceVote(vote) => Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: poll.clone().into(),
                resource_vote_choice: vote.resource_vote_choice(),
            })),
        }
    }

    #[wasm_bindgen(setter = choice)]
    pub fn set_choice(&mut self, choice: &ResourceVoteChoiceWasm) {
        self.0 = match self.0.clone() {
            Vote::ResourceVote(vote) => Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: vote.vote_poll().clone(),
                resource_vote_choice: choice.clone().into(),
            })),
        }
    }
}

impl_try_from_js_value!(VoteWasm, "Vote");
impl_wasm_conversions!(VoteWasm, Vote, VoteObjectJs, VoteJSONJs);
impl_wasm_type_info!(VoteWasm, Vote);
