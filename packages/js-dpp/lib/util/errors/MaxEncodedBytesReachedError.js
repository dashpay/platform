class MaxEncodedBytesReachedError extends Error {
  /**
   * @param {*} payload
   * @param {number} maxSizeKBytes
   */
  constructor(payload, maxSizeKBytes) {
    super();

    this.message = `Payload reached a ${maxSizeKBytes}Kb limit`;
    this.name = this.constructor.name;

    this.payload = payload;
    this.maxSizeKBytes = maxSizeKBytes;
  }

  /**
   * @return {*}
   */
  getPayload() {
    return this.payload;
  }

  /**
   * Get max payload size
   * @returns {number}
   */
  getMaxSizeKBytes() {
    return this.maxSizeKBytes;
  }
}

module.exports = MaxEncodedBytesReachedError;
