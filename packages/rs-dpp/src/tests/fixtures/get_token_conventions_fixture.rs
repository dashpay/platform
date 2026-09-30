use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use crate::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;

/// Token conventions with `entries` localizations: "en", then distinct
/// language codes, two letters while they last (2,704 of them) and three
/// after. Every form is "abc", the shortest `validate_localizations` accepts,
/// so each entry encodes in the fewest bytes a valid localization can: 13 for
/// a two-letter code.
pub fn get_token_conventions_with_localizations_fixture(
    entries: usize,
) -> TokenConfigurationConvention {
    let letters = || ('a'..='z').chain('A'..='Z');
    let two_letter_codes = letters().flat_map(move |a| letters().map(move |b| format!("{a}{b}")));
    let three_letter_codes = letters().flat_map(move |a| {
        letters().flat_map(move |b| letters().map(move |c| format!("{a}{b}{c}")))
    });
    let localizations = std::iter::once("en".to_string())
        .chain(
            two_letter_codes
                .chain(three_letter_codes)
                .filter(|code| code != "en"),
        )
        .take(entries)
        .map(|code| {
            (
                code,
                TokenConfigurationLocalizationV0 {
                    should_capitalize: false,
                    singular_form: "abc".to_string(),
                    plural_form: "abc".to_string(),
                }
                .into(),
            )
        })
        .collect();
    TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
        localizations,
        decimals: 8,
    })
}
