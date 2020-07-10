const ConsensusError = require('./ConsensusError');

class DocumentTimestampsMismatchError extends ConsensusError {
  /**
   * @param {DocumentCreateTransition} documentTransition
   */
  constructor(documentTransition) {
    super('Document createdAt and updatedAt timestamps are not equal');

    this.documentTransition = documentTransition;
  }

  /**
   * Get document create transition
   *
   * @return {DocumentCreateTransition}
   */
  getDocumentTransition() {
    return this.documentTransition;
  }
}

module.exports = DocumentTimestampsMismatchError;
