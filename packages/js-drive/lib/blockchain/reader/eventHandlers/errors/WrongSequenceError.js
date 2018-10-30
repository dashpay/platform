class WrongSequenceError extends Error {
  constructor() {
    super();

    this.name = this.constructor.name;
  }

  /**
   * @return {boolean}
   */
  // eslint-disable-next-line class-methods-use-this
  isFlowControl() {
    return true;
  }
}

module.exports = WrongSequenceError;
