const DPPError = require('../../errors/DPPError');

class DocumentAlreadyExistsError extends DPPError {
  /**
   * @param {DocumentCreateTransition} documentTransition
   */
  constructor(documentTransition) {
    super('Document already exists');

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

module.exports = DocumentAlreadyExistsError;
