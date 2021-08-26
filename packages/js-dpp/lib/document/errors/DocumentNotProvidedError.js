const DPPError = require('../../errors/DPPError');

class DocumentNotProvidedError extends DPPError {
  /**
   * @param {DocumentCreateTransition} documentTransition
   */
  constructor(documentTransition) {
    super('Document was not provided for apply of state transition');

    this.documentTransition = documentTransition;
  }

  /**
   * Get document transition
   *
   * @return {DocumentCreateTransition}
   */
  getDocumentTransition() {
    return this.documentTransition;
  }
}

module.exports = DocumentNotProvidedError;
