const ConsensusError = require('./ConsensusError');

const Document = require('../document/Document');

class DocumentNotFoundError extends ConsensusError {
  /**
   * @param {Document} document
   */
  constructor(document) {
    const noun = {
      [Document.ACTIONS.UPDATE]: 'Updated',
      [Document.ACTIONS.DELETE]: 'Deleted',
    };

    super(`${noun[document.getAction()]} Document not found`);

    this.document = document;
  }

  /**
   * Get Document
   *
   * @return {Document}
   */
  getDocument() {
    return this.document;
  }
}

module.exports = DocumentNotFoundError;
