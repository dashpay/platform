const AbstractBasicError = require('../AbstractBasicError');

class InvalidDocumentTransitionActionError extends AbstractBasicError {
  /**
   * @param {number} action
   */
  constructor(action) {
    super(`Document transition action ${action} is not supported`);

    this.action = action;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get action
   *
   * @return {*}
   */
  getAction() {
    return this.action;
  }
}

module.exports = InvalidDocumentTransitionActionError;
