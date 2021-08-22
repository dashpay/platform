use crate::identifier::Identifier;
use crate::metadata::Metadata;

pub struct DataContract {
    protocol_version: u32,
    id: Identifier,
    owner_id: Identifier,
    schema: String,
    documents: i32,
    definitions: i32,
    entropy: [byte],
    metadata: Metadata,
}

impl DataContract {
    fn is_document_defined(&self, documentType: &str) -> bool {
        // TODO: self.documents.has(documentType)
        false
    }

    fn set_document_type(&self, documentType: &str, documentSchema: &str) {
        // TODO: self.documents[type] = schema
    }

    fn get_document_schema(&self, documentType: &str) {
        // if (!this.isDocumentDefined(type)) {
        // throw new InvalidDocumentTypeError(type, this);
        // }
        //
        // return this.documents[type];
    }

}