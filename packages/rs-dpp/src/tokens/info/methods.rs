use crate::tokens::info::v0::IdentityTokenInfoV0Accessors;
use crate::tokens::info::IdentityTokenInfo;

impl IdentityTokenInfoV0Accessors for IdentityTokenInfo {
    fn frozen(&self) -> bool {
        match self {
            IdentityTokenInfo::V0(info) => info.frozen,
        }
    }

    fn set_frozen(&mut self, frozen: bool) {
        match self {
            IdentityTokenInfo::V0(info) => info.set_frozen(frozen),
        }
    }
}
