const AbstractResponse = require('../response/AbstractResponse');

class GetDocumentsResponse extends AbstractResponse {
  /**
   * @param {Buffer[]} documents
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(documents, metadata, proof = undefined) {
    super(metadata, proof);

    this.documents = documents;
  }

  /**
   * @returns {Buffer[]}
   */
  getDocuments() {
    return this.documents;
  }

  /**
   * @param proto
   * @returns {GetDocumentsResponse}
   */
  static createFromProto(proto) {
    const { metadata, proof } = AbstractResponse.createMetadataAndProofFromProto(proto);

    const documents = proto.getV0().getDocuments();

    return new GetDocumentsResponse(
      documents !== undefined
        ? documents.getDocumentsList().map((document) => Buffer.from(document)) : [],
      metadata,
      proof,
    );
  }
}

module.exports = GetDocumentsResponse;
