class RestartBlockchainReaderError extends Error {
  /**
   * @param {number} height
   */
  constructor(height) {
    super();

    this.height = height;
    this.name = this.constructor.name;
  }

  /**
   * @return {number}
   */
  getHeight() {
    return this.height;
  }

  /**
   * @return {boolean}
   */
  // eslint-disable-next-line class-methods-use-this
  isFlowControl() {
    return true;
  }
}

module.exports = RestartBlockchainReaderError;
