use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::voting::resource_vote_choice::ResourceVoteChoiceWasm;
use crate::voting::vote_poll::VotePollWasm;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
/**
 * ResourceVote serialized as a plain object.
 */
export interface ResourceVoteObject {
    votePoll: VotePollObject;
    resourceVoteChoice: ResourceVoteChoiceObject;
}

/**
 * ResourceVote serialized as JSON.
 */
export interface ResourceVoteJSON {
    votePoll: VotePollJSON;
    resourceVoteChoice: ResourceVoteChoiceJSON;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ResourceVoteObject")]
    pub type ResourceVoteObjectJs;

    #[wasm_bindgen(typescript_type = "ResourceVoteJSON")]
    pub type ResourceVoteJSONJs;
}

#[derive(Clone)]
#[wasm_bindgen(js_name = ResourceVote)]
pub struct ResourceVoteWasm(ResourceVote);

impl From<ResourceVote> for ResourceVoteWasm {
    fn from(vote: ResourceVote) -> Self {
        Self(vote)
    }
}

impl From<ResourceVoteWasm> for ResourceVote {
    fn from(vote: ResourceVoteWasm) -> Self {
        vote.0
    }
}

#[wasm_bindgen(js_class = ResourceVote)]
impl ResourceVoteWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(vote_poll: &VotePollWasm, choice: &ResourceVoteChoiceWasm) -> Self {
        ResourceVoteWasm(ResourceVote::V0(ResourceVoteV0 {
            vote_poll: vote_poll.clone().into(),
            resource_vote_choice: choice.clone().into(),
        }))
    }

    #[wasm_bindgen(getter = votePoll)]
    pub fn vote_poll(&self) -> VotePollWasm {
        self.0.vote_poll().clone().into()
    }

    #[wasm_bindgen(getter = choice)]
    pub fn choice(&self) -> ResourceVoteChoiceWasm {
        self.0.resource_vote_choice().into()
    }

    #[wasm_bindgen(setter = votePoll)]
    pub fn set_vote_poll(&mut self, vote_poll: &VotePollWasm) {
        let ResourceVote::V0(ResourceVoteV0 {
            resource_vote_choice,
            ..
        }) = self.0.clone();

        self.0 = ResourceVote::V0(ResourceVoteV0 {
            vote_poll: vote_poll.clone().into(),
            resource_vote_choice,
        });
    }

    #[wasm_bindgen(setter = choice)]
    pub fn set_choice(&mut self, choice: &ResourceVoteChoiceWasm) {
        let ResourceVote::V0(ResourceVoteV0 { vote_poll, .. }) = self.0.clone();

        self.0 = ResourceVote::V0(ResourceVoteV0 {
            vote_poll,
            resource_vote_choice: choice.clone().into(),
        });
    }
}

impl_wasm_conversions!(
    ResourceVoteWasm,
    ResourceVote,
    ResourceVoteObjectJs,
    ResourceVoteJSONJs
);
impl_wasm_type_info!(ResourceVoteWasm, ResourceVote);
