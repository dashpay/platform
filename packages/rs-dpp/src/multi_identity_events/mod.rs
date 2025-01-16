use platform_value::Identifier;
use std::collections::BTreeSet;

pub enum ActionTaker {
    SingleIdentity(Identifier),
    SpecifiedIdentities(BTreeSet<Identifier>),
}
