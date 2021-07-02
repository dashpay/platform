const AbstractResponse = require('../response/AbstractResponse');
const Metadata = require('../response/Metadata');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

class GetDocumentsResponse extends AbstractResponse {
  /**
   * @param {Buffer[]} documents
   * @param {Metadata} metadata
   */
  constructor(documents, metadata) {
    super(metadata);

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

    return new GetDocumentsResponse(
      proto.getDocumentsList().map((document) => Buffer.from(document)),
      new Metadata(metadata.toObject()),
    );
  }
}

module.exports = GetDocumentsResponse;
