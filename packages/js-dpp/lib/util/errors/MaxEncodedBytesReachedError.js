class MaxEncodedBytesReachedError extends Error {
  /**
   * @param {*} payload
   */
  constructor(payload) {
    super();

    this.message = 'Payload reached a 16Kb limit';
    this.name = this.constructor.name;

    this.payload = payload;
  }

  /**
   * @return {*}
   */
  getPayload() {
    return this.payload;
  }
}

module.exports = MaxEncodedBytesReachedError;
