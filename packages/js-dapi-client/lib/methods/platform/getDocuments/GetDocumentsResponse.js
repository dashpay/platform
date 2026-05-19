import AbstractResponse from '../response/AbstractResponse.js';

class GetDocumentsResponse extends AbstractResponse {
  /**
   * @param {Uint8Array[]} documents
   * @param {Metadata} metadata
   * @param {Proof} [proof]
   */
  constructor(documents, metadata, proof = undefined) {
    super(metadata, proof);

    this.documents = documents;
  }

  /**
   * @returns {Uint8Array[]}
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
        ? documents.getDocumentsList().map((document) => new Uint8Array(document)) : [],
      metadata,
      proof,
    );
  }
}

export default GetDocumentsResponse;
