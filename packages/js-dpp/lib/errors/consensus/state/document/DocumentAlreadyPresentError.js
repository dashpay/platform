const AbstractStateError = require('../AbstractStateError');
const Identifier = require('../../../../identifier/Identifier');

class DocumentAlreadyPresentError extends AbstractStateError {
  /**
   * @param {Buffer} documentId
   */
  constructor(documentId) {
    super(`Document ${Identifier.from(documentId)} is already present`);

    this.documentId = documentId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get document ID
   *
   * @return {Buffer}
   */
  getDocumentId() {
    return this.documentId;
  }
}

module.exports = DocumentAlreadyPresentError;
