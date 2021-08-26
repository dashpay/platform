const DPPError = require('../../errors/DPPError');

class InvalidActionNameError extends DPPError {
  /**
   * @param {string[]} actions
   */
  constructor(actions) {
    super('Invalid document action submitted');

    this.actions = actions;
  }

  /**
   * Get actions
   *
   * @returns {string[]}
   */
  getActions() {
    return this.actions;
  }
}

module.exports = InvalidActionNameError;
