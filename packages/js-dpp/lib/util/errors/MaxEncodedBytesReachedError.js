const DPPError = require('../../errors/DPPError');

class MaxEncodedBytesReachedError extends DPPError {
  /**
   * @param {*} payload
   * @param {number} maxSizeKBytes
   */
  constructor(payload, maxSizeKBytes) {
    super(`Payload reached a ${maxSizeKBytes}Kb limit`);

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
