const ConsensusError = require('./ConsensusError');

class InvalidDocumentTransitionEntropyError extends ConsensusError {
  /**
   * @param {
   *   RawDocumentCreateTransition|RawDocumentReplaceTransition|RawDocumentDeleteTransition
   * } rawDocumentTransition
   */
  constructor(rawDocumentTransition) {
    super('Invalid document transition entropy');

    this.rawDocumentTransition = rawDocumentTransition;
  }

  /**
   * Get raw document transition
   *
   * @return {
   *   RawDocumentCreateTransition|RawDocumentReplaceTransition|RawDocumentDeleteTransition
   * }
   */
  getRawDocumentTransition() {
    return this.rawDocumentTransition;
  }
}

module.exports = InvalidDocumentTransitionEntropyError;
