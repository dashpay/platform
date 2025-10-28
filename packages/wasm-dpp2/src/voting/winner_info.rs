use crate::identifier::IdentifierWasm;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Copy)]
#[wasm_bindgen(js_name = "ContestedDocumentVotePollWinnerInfo")]
pub struct ContestedDocumentVotePollWinnerInfoWasm(ContestedDocumentVotePollWinnerInfo);

impl From<ContestedDocumentVotePollWinnerInfo> for ContestedDocumentVotePollWinnerInfoWasm {
    fn from(info: ContestedDocumentVotePollWinnerInfo) -> Self {
        Self(info)
    }
}

impl From<ContestedDocumentVotePollWinnerInfoWasm> for ContestedDocumentVotePollWinnerInfo {
    fn from(info: ContestedDocumentVotePollWinnerInfoWasm) -> Self {
        info.0
    }
}

#[wasm_bindgen(js_class = ContestedDocumentVotePollWinnerInfo)]
impl ContestedDocumentVotePollWinnerInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        kind: &str,
        identity_id: Option<IdentifierWasm>,
    ) -> Result<ContestedDocumentVotePollWinnerInfoWasm, JsValue> {
        match kind {
            "none" | "NoWinner" | "NO_WINNER" => {
                Ok(ContestedDocumentVotePollWinnerInfo::NoWinner.into())
            }
            "identity" | "Identity" | "IDENTITY" => {
                let identity = identity_id.ok_or_else(|| {
                    JsValue::from_str("identityId is required when kind is 'identity'")
                })?;

                Ok(ContestedDocumentVotePollWinnerInfo::WonByIdentity(identity.into()).into())
            }
            "locked" | "Locked" | "LOCKED" => {
                Ok(ContestedDocumentVotePollWinnerInfo::Locked.into())
            }
            other => Err(JsValue::from_str(&format!(
                "Unsupported winner info kind '{}'",
                other
            ))),
        }
    }

    #[wasm_bindgen(getter = kind)]
    pub fn kind(&self) -> String {
        match self.0 {
            ContestedDocumentVotePollWinnerInfo::NoWinner => "none".to_string(),
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(_) => "identity".to_string(),
            ContestedDocumentVotePollWinnerInfo::Locked => "locked".to_string(),
        }
    }

    #[wasm_bindgen(getter = identityId)]
    pub fn identity_id(&self) -> Option<IdentifierWasm> {
        match self.0 {
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(identifier) => {
                Some(identifier.into())
            }
            _ => None,
        }
    }

    #[wasm_bindgen(js_name = "isLocked")]
    pub fn is_locked(&self) -> bool {
        matches!(self.0, ContestedDocumentVotePollWinnerInfo::Locked)
    }

    #[wasm_bindgen(js_name = "isWonByIdentity")]
    pub fn is_won_by_identity(&self) -> bool {
        matches!(
            self.0,
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(_)
        )
    }

    #[wasm_bindgen(js_name = "isNoWinner")]
    pub fn is_no_winner(&self) -> bool {
        matches!(self.0, ContestedDocumentVotePollWinnerInfo::NoWinner)
    }
}

impl ContestedDocumentVotePollWinnerInfoWasm {
    pub fn into_inner(self) -> ContestedDocumentVotePollWinnerInfo {
        self.0
    }

    pub fn as_inner(&self) -> ContestedDocumentVotePollWinnerInfo {
        self.0
    }
}
