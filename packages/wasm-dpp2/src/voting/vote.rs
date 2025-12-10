use crate::error::{WasmDppError, WasmDppResult};
use crate::voting::resource_vote_choice::ResourceVoteChoiceWasm;
use crate::voting::vote_poll::VotePollWasm;
use dpp::voting::votes::Vote;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use serde::Serialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name=Vote)]
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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "Vote".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "Vote".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(vote_poll: &VotePollWasm, resource_vote_choice: &ResourceVoteChoiceWasm) -> Self {
        VoteWasm(Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
            vote_poll: vote_poll.clone().into(),
            resource_vote_choice: resource_vote_choice.clone().into(),
        })))
    }

    #[wasm_bindgen(getter = votePoll)]
    pub fn vote_poll(&self) -> VotePollWasm {
        match &self.0 {
            Vote::ResourceVote(vote) => vote.vote_poll().clone().into(),
        }
    }

    #[wasm_bindgen(getter = resourceVoteChoice)]
    pub fn resource_vote_choice(&self) -> ResourceVoteChoiceWasm {
        match &self.0 {
            Vote::ResourceVote(vote) => vote.resource_vote_choice().into(),
        }
    }

    #[wasm_bindgen(setter = votePoll)]
    pub fn set_vote_poll(&mut self, vote_poll: &VotePollWasm) {
        self.0 = match self.0.clone() {
            Vote::ResourceVote(vote) => Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: vote_poll.clone().into(),
                resource_vote_choice: vote.resource_vote_choice(),
            })),
        }
    }

    #[wasm_bindgen(setter = resourceVoteChoice)]
    pub fn set_resource_vote_choice(&mut self, resource_vote_choice: &ResourceVoteChoiceWasm) {
        self.0 = match self.0.clone() {
            Vote::ResourceVote(vote) => Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: vote.vote_poll().clone(),
                resource_vote_choice: resource_vote_choice.clone().into(),
            })),
        }
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let json_value = serde_json::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        json_value
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<VoteWasm> {
        let json_value: JsonValue = serde_wasm_bindgen::from_value(js_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        let vote: Vote = serde_json::from_value(json_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(VoteWasm(vote))
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        serde_wasm_bindgen::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<VoteWasm> {
        let vote: Vote = serde_wasm_bindgen::from_value(js_value)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        Ok(VoteWasm(vote))
    }
}
