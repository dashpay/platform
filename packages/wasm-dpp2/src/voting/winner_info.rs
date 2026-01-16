use crate::identifier::IdentifierWasm;
use crate::impl_wasm_conversions;
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
            "NoWinner" | "noWinner" | "no_winner" | "none" | "NO_WINNER" => {
                Ok(ContestedDocumentVotePollWinnerInfo::NoWinner.into())
            }
            "WonByIdentity" | "wonByIdentity" | "won_by_identity" | "identity" | "Identity"
            | "IDENTITY" => {
                let identity = identity_id.ok_or_else(|| {
                    JsValue::from_str("identityId is required when kind is 'WonByIdentity'")
                })?;

                Ok(ContestedDocumentVotePollWinnerInfo::WonByIdentity(identity.into()).into())
            }
            "Locked" | "locked" | "LOCKED" => {
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
            ContestedDocumentVotePollWinnerInfo::NoWinner => "NoWinner".to_string(),
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(_) => "WonByIdentity".to_string(),
            ContestedDocumentVotePollWinnerInfo::Locked => "Locked".to_string(),
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

    #[wasm_bindgen(getter = "isLocked")]
    pub fn is_locked(&self) -> bool {
        matches!(self.0, ContestedDocumentVotePollWinnerInfo::Locked)
    }

    #[wasm_bindgen(getter = "isWonByIdentity")]
    pub fn is_won_by_identity(&self) -> bool {
        matches!(
            self.0,
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(_)
        )
    }

    #[wasm_bindgen(getter = "isNoWinner")]
    pub fn is_no_winner(&self) -> bool {
        matches!(self.0, ContestedDocumentVotePollWinnerInfo::NoWinner)
    }
}

impl_wasm_conversions!(
    ContestedDocumentVotePollWinnerInfoWasm,
    ContestedDocumentVotePollWinnerInfo
);

impl ContestedDocumentVotePollWinnerInfoWasm {
    pub fn into_inner(self) -> ContestedDocumentVotePollWinnerInfo {
        self.0
    }

    pub fn as_inner(&self) -> ContestedDocumentVotePollWinnerInfo {
        self.0
    }
}
