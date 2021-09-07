const AbstractStateError = require('../AbstractStateError');
const Identifier = require('../../../../identifier/Identifier');

class DocumentTimestampsMismatchError extends AbstractStateError {
  /**
   * @param {Buffer} documentId
   */
  constructor(documentId) {
    super(`Document ${Identifier.from(documentId)} createdAt and updatedAt timestamps are not equal`);

    this.documentId = documentId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get document create transition
   *
   * @return {Buffer}
   */
  getDocumentId() {
    return this.documentId;
  }
}

module.exports = DocumentTimestampsMismatchError;
