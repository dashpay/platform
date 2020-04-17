class InvalidActionNameError extends Error {
  /**
   * @param {string[]} actions
   */
  constructor(actions) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid document action submitted';

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
