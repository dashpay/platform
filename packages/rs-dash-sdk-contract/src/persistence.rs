//! Persistence semantics, stated as types so that generated wrappers and the
//! documentation cannot drift from the confirmed policy.
//!
//! Detached Rust values do not save automatically. Explicit insert and edit
//! operations, and the successful return of an exported mutable-receiver
//! wrapper, stage writes in the outer transaction. Nothing saves on `Drop`.
//! Document ids, revisions, owners and storage flags are host managed.

use crate::declare::Receiver;

/// The only points at which a write is staged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StagingPoint {
    /// `documents::<T>().insert(value)`: stages a native create, including
    /// index maintenance and validation.
    ExplicitInsert,
    /// `documents::<T>().edit(id, closure)`: loads one record, invokes the
    /// closure and stages the update only when the closure returns
    /// successfully.
    ExplicitEdit,
    /// An exported `&mut self` entry returning successfully: the generated
    /// wrapper loads exactly the addressed record, invokes the method and
    /// stages the receiver update. It never commits independently and never
    /// loads the whole collection.
    MutReceiverOnOk,
}

impl StagingPoint {
    /// Every staging point.
    pub const ALL: &'static [StagingPoint] = &[
        StagingPoint::ExplicitInsert,
        StagingPoint::ExplicitEdit,
        StagingPoint::MutReceiverOnOk,
    ];

    /// The staging point an entry receiver implies, if any.
    pub fn for_receiver(receiver: &Receiver) -> Option<StagingPoint> {
        match receiver {
            Receiver::Mut(_) => Some(StagingPoint::MutReceiverOnOk),
            Receiver::None | Receiver::Ref(_) => None,
        }
    }
}

/// Things that never stage a write.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NeverStages {
    /// Constructing or mutating a detached value.
    DetachedValue,
    /// Dropping a value, loaded or detached.
    Drop,
    /// A mutable reference escaping a call.
    EscapedReference,
    /// Calling an unmarked helper on a local value, whatever it mutates.
    UnmarkedHelper,
}

impl NeverStages {
    /// Every case.
    pub const ALL: &'static [NeverStages] = &[
        NeverStages::DetachedValue,
        NeverStages::Drop,
        NeverStages::EscapedReference,
        NeverStages::UnmarkedHelper,
    ];
}

/// Record attributes the host controls; a successful update cannot change
/// them through a mutable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HostManaged {
    /// The document id.
    DocumentId,
    /// The revision.
    Revision,
    /// The owner.
    Owner,
    /// Storage flags.
    StorageFlags,
}

impl HostManaged {
    /// Every host-managed attribute.
    pub const ALL: &'static [HostManaged] = &[
        HostManaged::DocumentId,
        HostManaged::Revision,
        HostManaged::Owner,
        HostManaged::StorageFlags,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::CollectionName;

    #[test]
    fn should_stage_only_on_a_mutable_receiver() {
        let scores = CollectionName::new("scores").unwrap();
        assert_eq!(
            StagingPoint::for_receiver(&Receiver::Mut(scores.clone())),
            Some(StagingPoint::MutReceiverOnOk)
        );
        assert_eq!(StagingPoint::for_receiver(&Receiver::Ref(scores)), None);
        assert_eq!(StagingPoint::for_receiver(&Receiver::None), None);
    }

    #[test]
    fn should_list_drop_among_the_cases_that_never_stage() {
        assert!(NeverStages::ALL.contains(&NeverStages::Drop));
        assert_eq!(StagingPoint::ALL.len(), 3);
        assert_eq!(HostManaged::ALL.len(), 4);
    }
}
