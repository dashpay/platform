use platform_value::Identifier;
use std::collections::BTreeSet;
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Eq)]
pub enum ActionGoal {
    ActionCompletion,
    ActionParticipation,
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Eq)]
pub enum ActionTaker {
    SingleIdentity(Identifier),
    SpecifiedIdentities(BTreeSet<Identifier>),
}
