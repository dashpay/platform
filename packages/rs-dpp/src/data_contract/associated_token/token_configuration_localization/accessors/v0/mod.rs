/// Accessor trait for getters of `TokenConfigurationLocalizationV0`
pub trait TokenConfigurationLocalizationV0Getters {
    /// Returns whether the token name should be capitalized.
    fn should_capitalize(&self) -> bool;

    /// Returns a reference to the singular form of the token name.
    fn singular_form(&self) -> &str;

    /// Returns a reference to the plural form of the token name.
    fn plural_form(&self) -> &str;
}

/// Accessor trait for setters of `TokenConfigurationLocalizationV0`
pub trait TokenConfigurationLocalizationV0Setters {
    /// Sets whether the token name should be capitalized.
    fn set_should_capitalize(&mut self, should_capitalize: bool);

    /// Sets the singular form of the token name.
    fn set_singular_form(&mut self, singular_form: String);

    /// Sets the plural form of the token name.
    fn set_plural_form(&mut self, plural_form: String);
}
