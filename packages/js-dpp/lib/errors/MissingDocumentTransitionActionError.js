const ConsensusError = require('./ConsensusError');

class MissingDocumentTransitionActionError extends ConsensusError {
  /**
   * @param {AbstractDocumentTransition} rawDocumentTransition
   */
  constructor(rawDocumentTransition) {
    super('$action is not present');

    this.rawDocumentTransition = rawDocumentTransition;
  }

  /**
   * Get raw Document
   *
   * @return {Object}
   */
  getRawDocumentTransition() {
    return this.rawDocumentTransition;
  }
}

module.exports = MissingDocumentTransitionActionError;
