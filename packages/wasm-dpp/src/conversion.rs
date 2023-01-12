use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversionOptions {
    #[serde(default)]
    pub skip_identifiers_conversion: bool,
}
