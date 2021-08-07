const AbstractResponse = require('../response/AbstractResponse');
const Metadata = require('../response/Metadata');
const InvalidResponseError = require('../response/errors/InvalidResponseError');
const createProofFromRawProof = require('../response/createProofFromRawProof');

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
    const metadata = proto.getMetadata();

    if (metadata === undefined) {
      throw new InvalidResponseError('Metadata is not defined');
    }

    const rawProof = proto.getProof();

    let proof;
    if (rawProof) {
      proof = createProofFromRawProof(rawProof);
    }

    return new GetDocumentsResponse(
      proto.getDocumentsList().map((document) => Buffer.from(document)),
      new Metadata(metadata.toObject()),
      proof,
    );
  }
}

module.exports = GetDocumentsResponse;
