const ConsensusError = require('./ConsensusError');

class InvalidDocumentTransitionActionError extends ConsensusError {
  /**
   * @param {*} action
   * @param {Object} rawDocumentTransition
   */
  constructor(action, rawDocumentTransition) {
    super(`Document transition action ${action} is not supported`);

    this.name = this.constructor.name;

    this.action = action;
    this.rawDocumentTransition = rawDocumentTransition;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get action
   *
   * @return {*}
   */
  getAction() {
    return this.action;
  }

  /**
   * Get document transition
   *
   * @return {Object}
   */
  getRawDocumentTransition() {
    return this.rawDocumentTransition;
  }
}

module.exports = InvalidDocumentTransitionActionError;
