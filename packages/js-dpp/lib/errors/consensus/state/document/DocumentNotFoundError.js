const AbstractStateError = require('../AbstractStateError');

const AbstractDocumentTransition = require('../../../../document/stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition');

class DocumentNotFoundError extends AbstractStateError {
  /**
   * @param {Buffer} documentId
   */
  constructor(documentId) {
    const noun = {
      [AbstractDocumentTransition.ACTIONS.REPLACE]: 'Updated',
      [AbstractDocumentTransition.ACTIONS.DELETE]: 'Deleted',
    };

    super(`${noun[documentId]} document not found`);

    this.documentId = documentId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get Document ID
   *
   * @return {Buffer}
   */
  getDocumentId() {
    return this.documentId;
  }
}

module.exports = DocumentNotFoundError;
