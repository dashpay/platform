use std::collections::BTreeSet;
use platform_value::Identifier;

pub enum ActionTaker {
    SingleIdentity(Identifier),
    SpecifiedIdentities(BTreeSet<Identifier>)
}