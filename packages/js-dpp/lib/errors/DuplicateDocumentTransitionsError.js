const ConsensusError = require('./ConsensusError');

class DuplicateDocumentTransitionsError extends ConsensusError {
  /**
   * @param {
   *   Array.<RawDocumentCreateTransition|RawDocumentReplaceTransition|RawDocumentDeleteTransition>
   * } rawDocumentTransitions
   */
  constructor(rawDocumentTransitions) {
    super('Duplicated document transitions found in state transition');

    this.rawDocumentTransitions = rawDocumentTransitions;
  }

  /**
   * Get duplicate raw transitions
   *
   * @return {
   *   Array.<RawDocumentCreateTransition|RawDocumentReplaceTransition|RawDocumentDeleteTransition>
   * }
   */
  getRawDocumentTransitions() {
    return this.rawDocumentTransitions;
  }
}

module.exports = DuplicateDocumentTransitionsError;
