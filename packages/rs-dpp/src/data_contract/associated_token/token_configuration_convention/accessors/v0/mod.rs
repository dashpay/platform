/// Accessor trait for getters of `TokenConfigurationConventionV0`
pub trait TokenConfigurationConventionV0Getters {
    /// Returns the localized token name in singular form
    fn singular_form_by_language_code_or_default(&self, language_code: &str) -> &str;
    /// Returns the localized token name in plural form
    fn plural_form_by_language_code_or_default(&self, language_code: &str) -> &str;
}
