const AbstractBasicError = require('../AbstractBasicError');

class InvalidDocumentTransitionActionError extends AbstractBasicError {
  /**
   * @param {*} action
   * @param {Object} rawDocumentTransition
   */
  constructor(action, rawDocumentTransition) {
    super(`Document transition action ${action} is not supported`);

    this.action = action;
    this.rawDocumentTransition = rawDocumentTransition;
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
