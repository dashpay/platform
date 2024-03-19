use crate::document::{DocumentV0, DocumentV0Getters, DocumentV0Setters};
use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentV0Getters for DocumentV0 {
    /// Returns the document's unique identifier.
    ///
    /// # Returns
    /// An `Identifier` representing the unique ID of the document.
    fn id(&self) -> Identifier {
        self.id
    }

    /// Returns the identifier of the document's owner.
    ///
    /// # Returns
    /// An `Identifier` representing the owner's ID.
    fn owner_id(&self) -> Identifier {
        self.owner_id
    }

    /// Provides a reference to the document's properties.
    ///
    /// # Returns
    /// A reference to a `BTreeMap<String, Value>` containing the document's properties.
    fn properties(&self) -> &BTreeMap<String, Value> {
        &self.properties
    }

    /// Provides a mutable reference to the document's properties.
    ///
    /// # Returns
    /// A mutable reference to a `BTreeMap<String, Value>` containing the document's properties.
    fn properties_mut(&mut self) -> &mut BTreeMap<String, Value> {
        &mut self.properties
    }

    /// Returns the document's revision, if it is part
    /// of the document. The document will have this field if it's schema has this document type
    /// as mutable.
    ///
    /// # Returns
    /// An `Option<Revision>` which is `Some(Revision)` if the document has a revision, or `None` if not.
    fn revision(&self) -> Option<Revision> {
        self.revision
    }

    /// Returns the timestamp of when the document was created, if it is part
    /// of the document. The document will have this field if it's schema has it set as required.
    ///
    /// # Returns
    /// An `Option<TimestampMillis>` representing the creation time in milliseconds, or `None` if not available.
    fn created_at(&self) -> Option<TimestampMillis> {
        self.created_at
    }

    /// Returns the timestamp of the last update to the document, if it is part
    /// of the document. The document will have this field if it's schema has it set as required.
    ///
    /// # Returns
    /// An `Option<TimestampMillis>` representing the update time in milliseconds, or `None` if not available.
    fn updated_at(&self) -> Option<TimestampMillis> {
        self.updated_at
    }

    /// Provides a reference to the document's unique identifier.
    ///
    /// # Returns
    /// A reference to an `Identifier` representing the unique ID of the document.
    fn id_ref(&self) -> &Identifier {
        &self.id
    }

    /// Provides a reference to the document's owner identifier.
    ///
    /// # Returns
    /// A reference to an `Identifier` representing the owner's ID.
    fn owner_id_ref(&self) -> &Identifier {
        &self.owner_id
    }

    /// Consumes the document and returns its properties.
    ///
    /// # Returns
    /// A `BTreeMap<String, Value>` containing the document's properties.
    fn properties_consumed(self) -> BTreeMap<String, Value> {
        self.properties
    }

    /// Returns the block height at which the document was created, if it is part
    /// of the document. The document will have this field if it's schema has it set as required.
    ///
    /// # Returns
    /// An `Option<u64>` representing the creation block height, or `None` if not available.
    fn created_at_block_height(&self) -> Option<u64> {
        self.created_at_block_height
    }

    /// Returns the block height at which the document was last updated, if it is part
    /// of the document. The document will have this field if it's schema has it set as required.
    ///
    /// # Returns
    /// An `Option<u64>` representing the update block height, or `None` if not available.
    fn updated_at_block_height(&self) -> Option<u64> {
        self.updated_at_block_height
    }

    /// Returns the core network block height at which the document was created, if it is part
    /// of the document. The document will have this field if it's schema has it set as required.
    ///
    /// # Returns
    /// An `Option<u32>` representing the creation core block height, or `None` if not available.
    fn created_at_core_block_height(&self) -> Option<u32> {
        self.created_at_core_block_height
    }

    /// Returns the core network block height at which the document was last updated, if it is part
    /// of the document. The document will have this field if it's schema has it set as required.
    ///
    /// # Returns
    /// An `Option<u32>` representing the update core block height, or `None` if not available.
    fn updated_at_core_block_height(&self) -> Option<u32> {
        self.updated_at_core_block_height
    }
}

impl DocumentV0Setters for DocumentV0 {
    /// Sets the document's unique identifier.
    ///
    /// # Parameters
    /// - `id`: An `Identifier` to set as the document's unique ID.
    fn set_id(&mut self, id: Identifier) {
        self.id = id;
    }

    /// Sets the identifier of the document's owner.
    ///
    /// # Parameters
    /// - `owner_id`: An `Identifier` to set as the document's owner ID.
    fn set_owner_id(&mut self, owner_id: Identifier) {
        self.owner_id = owner_id;
    }

    /// Sets the document's properties.
    ///
    /// # Parameters
    /// - `properties`: A `BTreeMap<String, Value>` containing the properties to set for the document.
    fn set_properties(&mut self, properties: BTreeMap<String, Value>) {
        self.properties = properties;
    }

    /// Sets the document's revision. This is applicable if the document's schema indicates
    /// the document type as mutable.
    ///
    /// # Parameters
    /// - `revision`: An `Option<Revision>` to set as the document's revision. `None` indicates
    /// the document does not have a revision.
    fn set_revision(&mut self, revision: Option<Revision>) {
        self.revision = revision;
    }

    /// Sets the timestamp of when the document was created. This is applicable if the document's
    /// schema requires a creation timestamp.
    ///
    /// # Parameters
    /// - `created_at`: An `Option<TimestampMillis>` to set as the document's creation timestamp.
    /// `None` indicates the timestamp is not available.
    fn set_created_at(&mut self, created_at: Option<TimestampMillis>) {
        self.created_at = created_at;
    }

    /// Sets the timestamp of the last update to the document. This is applicable if the document's
    /// schema requires an update timestamp.
    ///
    /// # Parameters
    /// - `updated_at`: An `Option<TimestampMillis>` to set as the document's last update timestamp.
    /// `None` indicates the timestamp is not available.
    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>) {
        self.updated_at = updated_at;
    }

    /// Sets the block height at which the document was created. This is applicable if the document's
    /// schema requires this information.
    ///
    /// # Parameters
    /// - `created_at_block_height`: An `Option<u64>` to set as the document's creation block height.
    /// `None` indicates the block height is not available.
    fn set_created_at_block_height(&mut self, created_at_block_height: Option<u64>) {
        self.created_at_block_height = created_at_block_height;
    }

    /// Sets the block height at which the document was last updated. This is applicable if the document's
    /// schema requires this information.
    ///
    /// # Parameters
    /// - `updated_at_block_height`: An `Option<u64>` to set as the document's last update block height.
    /// `None` indicates the block height is not available.
    fn set_updated_at_block_height(&mut self, updated_at_block_height: Option<u64>) {
        self.updated_at_block_height = updated_at_block_height;
    }

    /// Sets the core network block height at which the document was created. This is applicable if the
    /// document's schema requires this information.
    ///
    /// # Parameters
    /// - `created_at_core_block_height`: An `Option<u32>` to set as the document's creation core block height.
    /// `None` indicates the core block height is not available.
    fn set_created_at_core_block_height(&mut self, created_at_core_block_height: Option<u32>) {
        self.created_at_core_block_height = created_at_core_block_height;
    }

    /// Sets the core network block height at which the document was last updated. This is applicable if the
    /// document's schema requires this information.
    ///
    /// # Parameters
    /// - `updated_at_core_block_height`: An `Option<u32>` to set as the document's last update core block height.
    /// `None` indicates the core block height is not available.
    fn set_updated_at_core_block_height(&mut self, updated_at_core_block_height: Option<u32>) {
        self.updated_at_core_block_height = updated_at_core_block_height;
    }
}
