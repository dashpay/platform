const DPPError = require('../../errors/DPPError');

class InvalidDocumentActionError extends DPPError {
  /**
   * @param {
   *   DocumentCreateTransition|DocumentReplaceTransition|DocumentDeleteTransition
   * } documentTransition
   */
  constructor(documentTransition) {
    super(`Invalid Document action ${documentTransition.getAction()}`);

    this.documentTransition = documentTransition;
  }

  /**
   * Get Document transition
   *
   * @return {
   *   DocumentCreateTransition|DocumentReplaceTransition|DocumentDeleteTransition
   * }
   */
  getDocumentTransition() {
    return this.documentTransition;
  }
}

module.exports = InvalidDocumentActionError;
